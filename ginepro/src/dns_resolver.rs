//! Implements [`LookupService`] for dns.

use crate::{LookupService, ServiceDefinition};
use hickory_resolver::TokioResolver;
use std::{collections::HashSet, net::SocketAddr};

/// Implements [`LookupService`] by using DNS queries to lookup [`ServiceDefinition::hostname`].
pub struct DnsResolver {
    /// The trust-dns resolver which contacts the dns service directly such
    /// that we bypass os-specific dns caching.
    dns: TokioResolver,
}

impl DnsResolver {
    /// Construct a new [`DnsResolver`] from env and system configuration, e.g `resolv.conf`.
    pub async fn from_system_config() -> Result<Self, anyhow::Error> {
        let mut builder = TokioResolver::builder_tokio()?;

        // We do not want any caching on our side.
        let opts = builder.options_mut();
        opts.cache_size = 0;

        Ok(Self {
            dns: builder.build(),
        })
    }
}

#[async_trait::async_trait]
impl LookupService for DnsResolver {
    #[tracing::instrument(level = "debug", skip(self))]
    async fn resolve_service_endpoints(
        &self,
        definition: &ServiceDefinition,
    ) -> Result<HashSet<SocketAddr>, anyhow::Error> {
        match self.dns.lookup_ip(definition.hostname()).await {
            Ok(lookup) => {
                tracing::debug!("dns query expires in: {:?}", lookup.valid_until());
                Ok(lookup
                    .iter()
                    .map(|ip_addr| {
                        tracing::debug!("result: ip {}", ip_addr);
                        (ip_addr, definition.port()).into()
                    })
                    .collect())
            }
            Err(err) => Err(err.into()),
        }
    }
}
