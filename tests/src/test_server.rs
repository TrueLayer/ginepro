use futures::future::FutureExt;
use hyper::{Request, Response};
use std::{
    convert::Infallible,
    time::{Duration, Instant},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{
    body::Body,
    server::NamedService,
    service::Routes,
    transport::{
        server::{Router, Server},
        ServerTlsConfig,
    },
};
use tower_layer::Layer;
use tower_service::Service;

/// Manages construction and destruction of a tonic gRPC server for testing.
pub struct TestServer {
    shutdown_handle: Option<tokio::sync::oneshot::Sender<()>>,
    server_addr: String,
    server_future:
        Option<tokio::task::JoinHandle<std::result::Result<(), tonic::transport::Error>>>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Gracefully shutdown the gRPC Server.
        if let Some(sender) = self.shutdown_handle.take() {
            let _res = sender.send(());
        }
    }
}

impl TestServer {
    /// Bootstrap a tonic `TestServer`, with the provided `Service`.
    ///
    /// This function will run the server asynchronously, and
    /// tear it down when `Self` is dropped.
    pub async fn start<S, T: Into<Option<String>>>(
        service: S,
        address: T,
        tls: Option<ServerTlsConfig>,
    ) -> Self
    where
        S: Service<Request<Body>, Response = Response<Body>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + Sync
            + 'static,
        S::Future: Send + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
    {
        let mut server_builder = Server::builder();

        if let Some(config) = tls {
            server_builder = server_builder
                .tls_config(config)
                .expect("failed to set tls config");
        }

        Self::start_with_router(server_builder.add_service(service), address).await
    }

    /// Bootstrap a tonic `TestServer`, with the a tonic [`Router`].
    /// This enables you to construct a `TestServer` with multiple services.
    ///
    /// ```
    ///  use tests::test_server::TestServer;
    ///  use tonic::transport::Server;
    ///
    /// async fn build_test_server() {
    ///     let router = Server::builder()
    ///                     .add_service(tonic_health::server::health_reporter().1)
    ///                     .add_service(tonic_health::server::health_reporter().1);
    ///
    ///     TestServer::start_with_router(router, "localhost:9000".to_string()).await;
    ///  }
    ///
    /// ```
    ///
    /// This function will run the server asynchronously, and
    /// tear it down when `Self` is dropped.
    pub async fn start_with_router<L, T>(router: Router<L>, address: T) -> TestServer
    where
        L: Layer<Routes> + Send + 'static,
        L::Service: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
        <<L as Layer<Routes>>::Service as Service<Request<Body>>>::Future: Send + 'static,
        <<L as Layer<Routes>>::Service as Service<Request<Body>>>::Error:
            Into<Box<dyn std::error::Error + Send + Sync>> + Send,
        T: Into<Option<String>>,
    {
        let (shutdown_handle, shutdown) = tokio::sync::oneshot::channel::<()>();

        let listener =
            TcpListener::bind(address.into().unwrap_or_else(|| "127.0.0.1:0".to_string()))
                .await
                .expect("failed to bind tcplistener");
        let listener_addr = listener
            .local_addr()
            .expect("failed to retrieve sockeaddr from tokio listener");

        let server_addr = format!("127.0.0.1:{}", listener_addr.port());
        tracing::info!("server address: {}", server_addr);

        let server_future =
            tokio::spawn(router.serve_with_incoming_shutdown(
                TcpListenerStream::new(listener),
                shutdown.map(|_| ()),
            ));

        // await connectivity
        let wait_start = Instant::now();
        while let Err(e) = TcpStream::connect(listener_addr).await {
            if wait_start.elapsed() > Duration::from_secs(10) {
                panic!("Cannot connect to {listener_addr}: {e}");
            }
            tokio::task::yield_now().await;
        }

        TestServer {
            shutdown_handle: Some(shutdown_handle),
            server_addr,
            server_future: Some(server_future),
        }
    }

    /// Get the address `TestServer` is listening on.
    pub fn address(&self) -> &str {
        &self.server_addr
    }

    /// Shut the server down.
    pub async fn shutdown_sync(mut self) {
        // Gracefully shutdown the gRPC Server.
        if let Some(sender) = self.shutdown_handle.take() {
            let _res = sender.send(());
        }

        if let Some(server_future) = self.server_future.take() {
            server_future
                .await
                .expect("server did not exit gracefully")
                .expect("");
        }
    }
}
