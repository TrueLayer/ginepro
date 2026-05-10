use crate::{LookupService, ServiceDefinition};
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use tonic::transport::{
    channel::{Change, Endpoint},
    ClientTlsConfig,
};

#[derive(thiserror::Error, Debug)]
pub enum ProbeError {
    #[error("Failed to resolve ServiceDefinition")]
    ResolveServiceDefinition(#[source] anyhow::Error),
    #[error("Changeset sender closed")]
    ChangesetSenderClosed(#[source] anyhow::Error),
}

/// [`GrpcServiceProbe`] looks up IP addresses associated with the configured `host_name`
/// once every `probe_interval`.
/// If a new IP address is discovered or an old one disappears it notifies the [`tonic`] gRPC client.
///
/// The tonic load balancing side that is being notified work under the following assumptions:
///     * Tonic will never remove an endpoint from its set of servers to contact
///       unless we report a `Change::Remove` explicitly.
///     * Tonic does not perform any retries, which means that a subsequent call
///       after a failed one will succeed if there are enough servers, or if the
///       client manages to reconnect.
///     * Tonic will try reconnecting to a server if the connection to it breaks,
///       and we have not instructed the removal of that server's address from the
///       set of endpoints known to the tonic client.
///
pub struct GrpcServiceProbe<Lookup>
where
    Lookup: LookupService,
{
    service_definition: ServiceDefinition,
    scheme: http::uri::Scheme,
    dns_lookup: Lookup,
    probe_interval: tokio::time::Duration,
    endpoint_timeout: Option<tokio::time::Duration>,
    endpoint_connect_timeout: Option<tokio::time::Duration>,
    /// The set of last reported endpoints by `dns_lookup`.
    endpoints: HashSet<SocketAddr>,
    endpoint_reporter: Sender<Change<SocketAddr, Endpoint>>,
    tls_config: Option<ClientTlsConfig>,
}

/// Config parameters to customize the behavior of `GrpcServiceProbe`.
pub struct GrpcServiceProbeConfig<Lookup>
where
    Lookup: LookupService,
{
    /// the host name to resolve dns for and the service port.
    pub service_definition: ServiceDefinition,
    /// The lookup resolver.
    /// We are using a generic parameter and a trait constraint to allow mocking of DNS resolution in tests.
    pub dns_lookup: Lookup,
    /// How often the probe should update the ips.
    pub probe_interval: tokio::time::Duration,
    /// A timeout that will be applied to every endpoint.
    pub endpoint_timeout: Option<tokio::time::Duration>,
    /// A connection timeout that will be applied to every endpoint.
    pub endpoint_connect_timeout: Option<tokio::time::Duration>,
}

impl<Lookup: LookupService> GrpcServiceProbe<Lookup> {
    /// Construct `GrpcServiceProbe` with a `GrpcServiceProbeConfig` and
    /// the channel `endpoint_reporter` that will send endpoint changes.
    pub fn new_with_reporter(
        config: GrpcServiceProbeConfig<Lookup>,
        endpoint_reporter: Sender<Change<SocketAddr, Endpoint>>,
    ) -> GrpcServiceProbe<Lookup> {
        Self {
            service_definition: config.service_definition,
            dns_lookup: config.dns_lookup,
            probe_interval: config.probe_interval,
            endpoint_timeout: config.endpoint_timeout,
            endpoint_connect_timeout: config.endpoint_connect_timeout,
            endpoints: HashSet::new(),
            endpoint_reporter,
            scheme: http::uri::Scheme::HTTP,
            tls_config: None,
        }
    }

    /// Enable tls for all endpoints.
    pub fn with_tls(self, tls_config: ClientTlsConfig) -> GrpcServiceProbe<Lookup> {
        Self {
            tls_config: Some(tls_config),
            scheme: http::uri::Scheme::HTTPS,
            ..self
        }
    }

    /// Start probing the provided `hostname` for IP address changes.
    /// The probe loop exits once the receiving end of the tonic balance channel is closed,
    /// e.g. when the client has been dropped.
    /// Any other errors are seen as transient, and therefore retried after `self.probe_interval`.
    pub async fn probe(mut self) -> Result<(), anyhow::Error> {
        let client_closed_err = || {
            anyhow::anyhow!(
                "The gRPC client has closed the channel therefore the DNS probe loop will exit."
            )
        };

        loop {
            // If the receiver is already gone (e.g. client dropped), exit promptly.
            if self.endpoint_reporter.is_closed() {
                return Err(client_closed_err());
            }

            self.probe_once().await.or_else(|err| {
                // Only terminate if the changeset channel has been closed.
                if let ProbeError::ChangesetSenderClosed(_) = err {
                    Err(err)
                } else {
                    Ok(())
                }
            })?;

            tokio::select! {
                _ = self.endpoint_reporter.closed() => return Err(client_closed_err()),
                _ = tokio::time::sleep(self.probe_interval) => {},
            }
        }
    }

    /// Update tonic with a set of IPs that are retrieved by querying `hostname`.
    pub async fn probe_once(&mut self) -> Result<(), ProbeError> {
        match self
            .dns_lookup
            .resolve_service_endpoints(&self.service_definition)
            .await
        {
            Ok(endpoints) => {
                let changeset = self.create_changeset(&endpoints).await;

                // Report the changeset to `tonic` and commit the new endpoints
                // if we succeed to report the changeset.
                self.report_and_commit(changeset, endpoints).await.map_err(|e| {
                        tracing::error!("Failed to report the discovered DNS changeset. The gRPC client has closed the channel therefore the DNS probe loop will exit.\n{:?}", e);
                        e
                    })?;
            }
            Err(err) => {
                return Err(ProbeError::ResolveServiceDefinition(
                    err.context("failed to resolve ips from host"),
                ));
            }
        }

        Ok(())
    }

    /// Construct a changeset and report the endpoint changes to tonic.
    async fn create_changeset(
        &mut self,
        endpoints: &HashSet<SocketAddr>,
    ) -> Vec<Change<SocketAddr, Endpoint>> {
        let mut changeset = Vec::new();

        let remove_set: HashSet<SocketAddr> =
            self.endpoints.difference(endpoints).copied().collect();

        let add_set: HashSet<SocketAddr> = endpoints.difference(&self.endpoints).copied().collect();

        changeset.extend(
            add_set
                .into_iter()
                .filter_map(|addr| self.build_endpoint(&addr).map(|endpoint| (addr, endpoint)))
                .map(|(addr, endpoint)| Change::Insert(addr, endpoint)),
        );

        changeset.extend(remove_set.into_iter().map(Change::Remove));

        changeset
    }

    /// Update the endpoint working set to be equal to the result of the last probe.
    fn overwrite_endpoints(&mut self, current_ips: HashSet<SocketAddr>) {
        self.endpoints = current_ips;
    }

    /// Report `changeset` to the gRPC client and commit the changes
    /// by setting the new working set to the most recent list of endpoints.
    ///
    /// Function fails if the `Sender` is closed.
    #[tracing::instrument(
        skip(endpoints, self),
        level = "debug",
        name = "report-and-commit-endpoint-changeset"
    )]
    async fn report_and_commit(
        &mut self,
        changeset: Vec<Change<SocketAddr, Endpoint>>,
        endpoints: HashSet<SocketAddr>,
    ) -> Result<(), ProbeError> {
        for change in changeset {
            if self.endpoint_reporter.send(change).await.is_err() {
                return Err(ProbeError::ChangesetSenderClosed(anyhow::anyhow!("Tried to report endpoint changes on a closed channel, this is probably due to the gRPC client being dropped.")));
            }
        }

        // When we reach this point we have sent all the changes to the client
        // and can overwrite the endpoints.
        // If we failed earlier the client died so we're in the clear!
        self.overwrite_endpoints(endpoints);

        Ok(())
    }

    fn build_endpoint(&self, ip_address: &SocketAddr) -> Option<Endpoint> {
        let uri = match ip_address.is_ipv6() {
            false => format!(
                "{}://{}:{}",
                self.scheme,
                ip_address.ip(),
                ip_address.port()
            ),
            true => format!(
                "{}://[{}]:{}",
                self.scheme,
                ip_address.ip(),
                ip_address.port()
            ),
        };

        let mut endpoint = Endpoint::from_shared(uri)
            .map_err(|err| {
                tracing::warn!("endpoint creation error: {:?}", err);
            })
            .ok()?;

        if let Some(ref tls_config) = self.tls_config {
            endpoint = endpoint
                .tls_config(tls_config.clone())
                .map_err(|err| {
                    tracing::warn!("tls error: {:?}", err);
                    err
                })
                .ok()?;
        }

        if let Some(ref timeout) = self.endpoint_timeout {
            endpoint = endpoint.timeout(*timeout);
        }
        if let Some(ref connect_timeout) = self.endpoint_connect_timeout {
            endpoint = endpoint.connect_timeout(*connect_timeout)
        }

        Some(endpoint)
    }
}
