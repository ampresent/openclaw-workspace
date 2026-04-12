use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug, Clone)]
#[command(name = "UtopOS-agent", about = "NixOS diagnostic agent for AI agents")]
pub struct Config {
    /// Bind address
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Bind port
    #[arg(long, default_value_t = 7890)]
    pub port: u16,

    /// NixOS config directory
    #[arg(long, default_value = "/etc/nixos")]
    pub nixos_dir: String,

    /// Max log lines to return
    #[arg(long, default_value_t = 200)]
    pub max_log_lines: usize,

    /// API token for authentication (optional). If set, requests must include
    /// Authorization: Bearer <token> header.
    #[arg(long, env = "NIX_EVO_TOKEN")]
    pub api_token: Option<String>,
}

impl Config {
    pub fn from_args() -> Self {
        Self::parse()
    }

    pub fn bind_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("invalid bind address")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            host: "127.0.0.1".into(),
            port: 7890,
            nixos_dir: "/etc/nixos".into(),
            max_log_lines: 200,
            api_token: None,
        }
    }

    #[test]
    fn test_default_bind_addr() {
        let config = test_config();
        let addr = config.bind_addr();
        assert_eq!(addr.port(), 7890);
        assert!(addr.is_ipv4());
    }

    #[test]
    fn test_custom_port() {
        let config = Config {
            host: "0.0.0.0".into(),
            port: 8080,
            nixos_dir: "/etc/nixos".into(),
            max_log_lines: 100,
            api_token: None,
        };
        let addr = config.bind_addr();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_config_clone() {
        let config = test_config();
        let cloned = config.clone();
        assert_eq!(cloned.host, config.host);
        assert_eq!(cloned.port, config.port);
        assert_eq!(cloned.nixos_dir, config.nixos_dir);
    }

    #[test]
    fn test_config_debug() {
        let config = test_config();
        let debug = format!("{config:?}");
        assert!(debug.contains("127.0.0.1"));
        assert!(debug.contains("7890"));
    }

    #[test]
    fn test_api_token_none() {
        let config = test_config();
        assert!(config.api_token.is_none());
    }

    #[test]
    fn test_api_token_some() {
        let config = Config {
            host: "127.0.0.1".into(),
            port: 7890,
            nixos_dir: "/etc/nixos".into(),
            max_log_lines: 200,
            api_token: Some("secret123".into()),
        };
        assert_eq!(config.api_token.as_deref(), Some("secret123"));
    }

    #[test]
    fn test_max_log_lines_default() {
        let config = test_config();
        assert_eq!(config.max_log_lines, 200);
    }

    #[test]
    fn test_ipv6_bind_addr() {
        let config = Config {
            host: "::1".into(),
            port: 9090,
            nixos_dir: "/etc/nixos".into(),
            max_log_lines: 200,
            api_token: None,
        };
        let addr = config.bind_addr();
        assert_eq!(addr.port(), 9090);
        assert!(addr.is_ipv6());
    }
}
