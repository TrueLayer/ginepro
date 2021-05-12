//! Defines the interface that [`LoadBalancedChannel`](crate::LoadBalancedChannel) requires in order
//! to resolve all the IP adresses for a given service.

use std::{collections::HashSet, net::SocketAddr};

use crate::ServiceDefinition;

/// Interface that provides functionality to
/// acquire a list of ips given a valid host name.
#[async_trait::async_trait]
pub trait LookupService {
    /// Return a list of unique [`SocketAddr`] associated with the provided
    /// [`ServiceDefinition`](crate::ServiceDefinition) containing the `hostname` `port` of the service.
    /// If no ip addresses were resolved, an empty HashSet is returned.
    async fn resolve_service_endpoints(
        &self,
        definition: &ServiceDefinition,
    ) -> Result<HashSet<SocketAddr>, anyhow::Error>;
}
