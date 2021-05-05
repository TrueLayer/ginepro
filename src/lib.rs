//! This module contains all gRPC client-specific functionality.

mod client_channel;
mod resolve;
mod service;
mod service_probe;

pub mod pb {
    // Exposes a `Test` grpc service definition for use in testing.
    tonic::include_proto!("test");
}

pub use client_channel::*;
pub use resolve::*;
pub use service::*;
pub use service_probe::*;
