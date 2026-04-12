//! TLS configuration for the agent.
//!
//! Provides optional HTTPS support with auto-generated self-signed certificates
//! for local development and certificate file support for production.
//!
//! ## Design
//!
//! TLS is optional. For local use (127.0.0.1), HTTP is fine.
//! For remote access, use TLS or SSH tunnels.
//!
//! ## Usage
//!
//! ```bash
//! # Auto-generate self-signed cert (dev only)
//! UtopOS-agent --tls-auto
//!
//! # Use existing certs
//! UtopOS-agent --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem
//! ```

/// TLS configuration from CLI args
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to TLS certificate file (PEM)
    pub cert_path: Option<String>,
    /// Path to TLS private key file (PEM)
    pub key_path: Option<String>,
    /// Auto-generate self-signed certificate (for development)
    pub auto_generate: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: None,
            key_path: None,
            auto_generate: false,
        }
    }
}

impl TlsConfig {
    pub fn is_enabled(&self) -> bool {
        self.cert_path.is_some() || self.auto_generate
    }
}

/// Check if TLS is configured
pub fn tls_available(config: &TlsConfig) -> bool {
    config.is_enabled()
}

// Note: Actual TLS listener setup requires rustls or native-tls.
// This module provides the configuration structure. The actual
// implementation depends on the TLS library chosen.
//
// For axum 0.8 + rustls:
//   let tls_config = RustlsConfig::from_pem_file(cert, key).await?;
//   axum_server::bind_rustls(addr, tls_config).serve(app).await?;
//
// For self-signed cert generation:
//   Use rcgen crate to generate certificates at startup.
