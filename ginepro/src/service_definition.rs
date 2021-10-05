use std::convert::TryFrom;

use anyhow::Context;

/// Defines a gRPC service with a `hostname` and a `port`.
/// The hostname will be resolved to the concrete ips of the service servers.
#[derive(Debug)]
pub struct ServiceDefinition {
    /// The hostname of the service.
    hostname: String,
    /// The service port.
    port: u16,
}

impl ServiceDefinition {
    /// Create a [`ServiceDefinition`] from a valid `hostname` and `port`.
    ///
    /// This function will fail is the `hostname` is not a valid domain name.
    pub fn from_parts<T: ToString>(hostname: T, port: u16) -> Result<Self, anyhow::Error> {
        let hostname = hostname.to_string();

        trust_dns_resolver::Name::from_utf8(&hostname)
            .map_err(anyhow::Error::from)
            .context("invalid 'hostname'")?;

        Ok(Self { hostname, port })
    }

    /// Get the `hostname` part of a `ServiceDefinition`.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Get the `port` part of a `ServiceDefinition`.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl TryFrom<(&str, u16)> for ServiceDefinition {
    type Error = anyhow::Error;

    fn try_from((hostname, port): (&str, u16)) -> Result<Self, Self::Error> {
        Self::from_parts(hostname, port)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prop_compose;

    prop_compose! {
        fn valid_hostname()(s in "[a-z.0-9*A-Z]") -> String {
            s
        }
    }

    prop_compose! {
        fn invalid_hostname()(s in "[^a-z.0-9*A-Z]+") -> String {
            s
        }
    }

    proptest::proptest! {
        #[test]
        fn valid_hostname_shall_succeed(hostname in valid_hostname()) {
                proptest::prop_assert!(ServiceDefinition::from_parts(hostname, 5000).is_ok());
        }

        #[test]
        fn invalid_hostname_shall_fail(hostname in invalid_hostname()) {
                proptest::prop_assert!(ServiceDefinition::from_parts(hostname, 5000).is_err());
        }
    }
}
