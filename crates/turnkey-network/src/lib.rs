//! Network communication layer for Turnkey
//!
//! This crate provides TCP client and server implementations for the Henry protocol.
//! It handles network transport, connection management, and integrates with the
//! HenryCodec for automatic message framing.
//!
//! # Components
//!
//! - **TcpClient**: Client for connecting to validation servers (Issue #65)
//! - **TcpServer**: Server for accepting emulator connections (Issue #66 - future)
//!
//! # Example
//!
//! ```no_run
//! use turnkey_network::{TcpClient, TcpClientConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = TcpClientConfig {
//!     server_addr: "127.0.0.1:3000".parse()?,
//!     timeout: Duration::from_millis(3000),
//! };
//!
//! let mut client = TcpClient::new(config);
//! client.connect().await?;
//! # Ok(())
//! # }
//! ```

mod client;

pub use client::{TcpClient, TcpClientConfig, TcpClientError};
