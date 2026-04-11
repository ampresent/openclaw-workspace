use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug, Clone)]
#[command(name = "nix-evo-agent", about = "NixOS diagnostic agent for AI agents")]
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
