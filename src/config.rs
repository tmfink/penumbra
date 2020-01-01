use std::net::IpAddr;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HTTPSConfig {
    pub ports: ProtoPorts,
    pub _tls_cert: Vec<u8>,
    pub _tls_key: Vec<u8>,
}

/// Represents a listen/connect port number for a protocol
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ProtoPorts {
    pub listen: u16,
    pub connect: u16,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UmbraOptions {
    pub http_ports: Option<ProtoPorts>,
    pub listen_ip: IpAddr,
    pub connect_ip: IpAddr,
    pub https: Option<HTTPSConfig>,
    pub _config: Vec<u8>,
}