/// Defines a gRPC service with a `hostname` and a `port`.
/// The hostname will be resolved to the concrete ips of the service servers.
#[derive(Debug)]
pub struct ServiceDefinition {
    /// The hostname of the service.
    pub hostname: String,
    /// The service port.
    pub port: u16,
}

impl From<(&str, u16)> for ServiceDefinition {
    fn from(service: (&str, u16)) -> Self {
        Self {
            hostname: service.0.to_string(),
            port: service.1,
        }
    }
}
