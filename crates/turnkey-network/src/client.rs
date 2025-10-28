//! TCP Client for Henry protocol communication.
//!
//! This module provides a simple TCP client for the turnstile emulator to connect
//! to validation servers and exchange Henry protocol messages. The client uses
//! the HenryCodec for automatic message encoding/decoding with Tokio's async I/O.
//!
//! # Architecture
//!
//! ```text
//! TurnstileEmulator
//!     │
//!     ├─> OnlineValidator
//!     │       │
//!     │       └─> TcpClient ───(TCP)───> Validation Server
//!     │              │
//!     │              └─> HenryCodec (automatic framing)
//! ```
//!
//! # Example Usage
//!
//! ```no_run
//! use turnkey_network::{TcpClient, TcpClientConfig};
//! use turnkey_protocol::{MessageBuilder, CommandCode};
//! use turnkey_core::DeviceId;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure client
//! let config = TcpClientConfig {
//!     server_addr: "192.168.0.100:3000".parse()?,
//!     timeout: Duration::from_millis(3000),
//! };
//!
//! // Create and connect
//! let mut client = TcpClient::new(config);
//! client.connect().await?;
//!
//! // Send access request
//! let device_id = DeviceId::new(15)?;
//! let message = MessageBuilder::new(device_id, CommandCode::AccessRequest)
//!     .build()?;
//! client.send(message).await?;
//!
//! // Receive response
//! let response = client.recv().await?;
//! println!("Received: {:?}", response);
//!
//! // Clean up
//! client.close().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Design Principles
//!
//! The TcpClient is designed as a simple transport layer:
//! - **No automatic retry**: Caller decides retry strategy
//! - **No connection pooling**: Single connection per turnstile
//! - **No keepalive**: Short-lived connections
//! - **Simple error handling**: Clear errors, no recovery
//!
//! This keeps the client focused and testable, pushing business logic
//! to higher layers like OnlineValidator.
//!
//! # Timeout Handling
//!
//! All I/O operations have configurable timeouts (default: 3000ms):
//! - Connection timeout
//! - Send timeout
//! - Receive timeout
//!
//! Timeout errors are returned to the caller for appropriate handling.
//!
//! # Related
//!
//! - Issue #65: TCP Client implementation
//! - See `docs/tcp-client-detailed-spec.md` for complete specification

use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, trace, warn};
use turnkey_protocol::{HenryCodec, Message};

/// Configuration for TCP client
///
/// # Example
///
/// ```
/// use turnkey_network::TcpClientConfig;
/// use std::time::Duration;
///
/// let config = TcpClientConfig {
///     server_addr: "127.0.0.1:3000".parse().unwrap(),
///     timeout: Duration::from_millis(5000),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TcpClientConfig {
    /// Server address to connect to
    pub server_addr: SocketAddr,

    /// Timeout for all I/O operations (connect, send, recv)
    pub timeout: Duration,
}

impl Default for TcpClientConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:3000".parse().unwrap(),
            timeout: Duration::from_millis(3000),
        }
    }
}

/// Errors that can occur during TCP client operations
#[derive(Debug, Error)]
pub enum TcpClientError {
    /// Client is not connected to server
    #[error("Not connected to server")]
    NotConnected,

    /// Connection attempt timed out
    #[error("Connection timeout after {0}ms")]
    ConnectionTimeout(u64),

    /// Read operation timed out
    #[error("Read timeout after {0}ms")]
    ReadTimeout(u64),

    /// Write operation timed out
    #[error("Write timeout after {0}ms")]
    WriteTimeout(u64),

    /// Connection was lost during operation
    #[error("Connection lost: {0}")]
    ConnectionLost(String),

    /// Protocol-level error from HenryCodec
    #[error("Protocol error: {0}")]
    Protocol(#[from] turnkey_core::Error),

    /// Low-level I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Codec error during message encoding/decoding
    #[error("Codec error: {0}")]
    Codec(String),
}

/// TCP client for Henry protocol communication
///
/// The `TcpClient` provides a simple interface for connecting to validation
/// servers and exchanging Henry protocol messages. It handles connection
/// management, message framing via HenryCodec, and timeout enforcement.
///
/// # Connection Lifecycle
///
/// 1. Create client with `new()`
/// 2. Connect to server with `connect()`
/// 3. Exchange messages with `send()` and `recv()`
/// 4. Close connection with `close()`
///
/// # Thread Safety
///
/// `TcpClient` is not `Send` or `Sync` by design. Each turnstile emulator
/// should have its own client instance on a single task.
///
/// # Example
///
/// ```no_run
/// use turnkey_network::{TcpClient, TcpClientConfig};
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = TcpClientConfig {
///     server_addr: "127.0.0.1:3000".parse()?,
///     timeout: Duration::from_millis(3000),
/// };
///
/// let mut client = TcpClient::new(config);
/// client.connect().await?;
/// assert!(client.is_connected());
///
/// // ... use client ...
///
/// client.close().await?;
/// assert!(!client.is_connected());
/// # Ok(())
/// # }
/// ```
pub struct TcpClient {
    /// Server address to connect to
    server_addr: SocketAddr,

    /// Framed TCP stream with HenryCodec (None if not connected)
    framed: Option<Framed<TcpStream, HenryCodec>>,

    /// Timeout for all I/O operations
    timeout: Duration,
}

impl TcpClient {
    /// Create a new TCP client with the given configuration
    ///
    /// The client is not connected after creation. Call `connect()` to
    /// establish a connection.
    ///
    /// # Example
    ///
    /// ```
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    ///
    /// let client = TcpClient::new(TcpClientConfig::default());
    /// assert!(!client.is_connected());
    /// ```
    pub fn new(config: TcpClientConfig) -> Self {
        debug!("Creating TCP client for server {}", config.server_addr);

        Self {
            server_addr: config.server_addr,
            framed: None,
            timeout: config.timeout,
        }
    }

    /// Connect to the validation server
    ///
    /// Establishes a TCP connection to the configured server address with
    /// timeout. The connection is configured with TCP_NODELAY for low latency.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Connection times out
    /// - Server refuses connection
    /// - Network is unreachable
    /// - Invalid address
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    /// assert!(client.is_connected());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&mut self) -> Result<(), TcpClientError> {
        info!("Connecting to server at {}", self.server_addr);

        // Attempt connection with timeout
        let stream =
            match tokio::time::timeout(self.timeout, TcpStream::connect(self.server_addr)).await {
                Ok(Ok(stream)) => {
                    info!("Successfully connected to {}", self.server_addr);
                    stream
                }
                Ok(Err(e)) => {
                    error!("Connection failed: {}", e);
                    return Err(e.into());
                }
                Err(_) => {
                    warn!("Connection timeout after {}ms", self.timeout.as_millis());
                    return Err(TcpClientError::ConnectionTimeout(
                        self.timeout.as_millis() as u64
                    ));
                }
            };

        // Configure TCP_NODELAY to disable Nagle's algorithm.
        // Critical for Henry protocol latency: access requests must be processed
        // within 3000ms timeout window. Nagle's algorithm could introduce
        // 40-200ms delays waiting for more data before sending packets.
        if let Err(e) = stream.set_nodelay(true) {
            warn!("Failed to set TCP_NODELAY: {} - latency may be impacted", e);
        }

        // Wrap stream with HenryCodec for automatic framing
        self.framed = Some(Framed::new(stream, HenryCodec::new()));

        debug!("Client connected and ready");
        Ok(())
    }

    /// Send a message to the server
    ///
    /// Encodes the message using HenryCodec and sends it to the server
    /// with timeout enforcement.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client is not connected
    /// - Send operation times out
    /// - Connection is lost
    /// - Message encoding fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    /// use turnkey_protocol::{MessageBuilder, CommandCode};
    /// use turnkey_core::DeviceId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    ///
    /// let device_id = DeviceId::new(15)?;
    /// let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
    ///     .build()?;
    ///
    /// client.send(message).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(&mut self, message: Message) -> Result<(), TcpClientError> {
        trace!(
            device_id = %message.device_id,
            command = ?message.command,
            field_count = message.fields.len(),
            "Sending message to server"
        );

        // Check if connected
        let framed = self.framed.as_mut().ok_or(TcpClientError::NotConnected)?;

        // Send message with timeout
        match tokio::time::timeout(self.timeout, framed.send(message)).await {
            Ok(Ok(())) => {
                trace!("Message sent successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Failed to send message: {}", e);
                Err(TcpClientError::Protocol(e))
            }
            Err(_) => {
                warn!("Send timeout after {}ms", self.timeout.as_millis());
                Err(TcpClientError::WriteTimeout(self.timeout.as_millis() as u64))
            }
        }
    }

    /// Receive a message from the server
    ///
    /// Waits for a complete message from the server with timeout.
    /// The message is automatically decoded by HenryCodec.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client is not connected
    /// - Receive operation times out
    /// - Connection is lost
    /// - Message decoding fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    ///
    /// let response = client.recv().await?;
    /// println!("Received: {:?}", response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn recv(&mut self) -> Result<Message, TcpClientError> {
        trace!("Waiting for message from server");

        // Check if connected
        let framed = self.framed.as_mut().ok_or(TcpClientError::NotConnected)?;

        // Receive message with timeout
        match tokio::time::timeout(self.timeout, framed.next()).await {
            Ok(Some(Ok(message))) => {
                trace!(
                    device_id = %message.device_id,
                    command = ?message.command,
                    field_count = message.fields.len(),
                    "Received message from server"
                );
                Ok(message)
            }
            Ok(Some(Err(e))) => {
                error!("Failed to decode message: {}", e);
                Err(TcpClientError::Protocol(e))
            }
            Ok(None) => {
                warn!("Connection closed by server");
                Err(TcpClientError::ConnectionLost(
                    "Server closed connection".to_string(),
                ))
            }
            Err(_) => {
                warn!("Receive timeout after {}ms", self.timeout.as_millis());
                Err(TcpClientError::ReadTimeout(self.timeout.as_millis() as u64))
            }
        }
    }

    /// Check if client is connected to server
    ///
    /// Returns `true` if the client has an active connection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// assert!(!client.is_connected());
    ///
    /// client.connect().await?;
    /// assert!(client.is_connected());
    ///
    /// client.close().await?;
    /// assert!(!client.is_connected());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_connected(&self) -> bool {
        self.framed.is_some()
    }

    /// Close the connection gracefully
    ///
    /// Closes the TCP connection and cleans up resources. This method
    /// is idempotent - calling it multiple times is safe.
    ///
    /// Flush and shutdown operations have a 500ms timeout each to prevent
    /// hanging if the network is down or unresponsive.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be closed cleanly,
    /// though the connection will still be dropped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    ///
    /// // ... use client ...
    ///
    /// client.close().await?;
    /// assert!(!client.is_connected());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(&mut self) -> Result<(), TcpClientError> {
        if let Some(mut framed) = self.framed.take() {
            info!("Closing connection to {}", self.server_addr);

            // Flush with timeout to prevent hanging on network issues
            let flush_timeout = Duration::from_millis(500);
            match tokio::time::timeout(flush_timeout, framed.flush()).await {
                Ok(Ok(())) => {
                    debug!("Flush completed successfully");
                }
                Ok(Err(e)) => {
                    warn!("Error flushing during close: {}", e);
                }
                Err(_) => {
                    warn!(
                        "Flush timeout during close ({}ms)",
                        flush_timeout.as_millis()
                    );
                }
            }

            // Get the underlying stream and shutdown with timeout
            let mut stream = framed.into_inner();
            let shutdown_timeout = Duration::from_millis(500);
            match tokio::time::timeout(shutdown_timeout, stream.shutdown()).await {
                Ok(Ok(())) => {
                    debug!("Shutdown completed successfully");
                }
                Ok(Err(e)) => {
                    warn!("Error during shutdown: {}", e);
                }
                Err(_) => {
                    warn!(
                        "Shutdown timeout during close ({}ms)",
                        shutdown_timeout.as_millis()
                    );
                }
            }

            debug!("Connection closed");
        }

        Ok(())
    }
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        if self.framed.is_some() {
            debug!("TcpClient dropped while connected - connection will be closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turnkey_core::DeviceId;
    use turnkey_protocol::{CommandCode, MessageBuilder};

    #[test]
    fn test_config_default() {
        let config = TcpClientConfig::default();
        assert_eq!(config.server_addr.port(), 3000);
        assert_eq!(config.timeout.as_millis(), 3000);
    }

    #[test]
    fn test_client_creation() {
        let config = TcpClientConfig::default();
        let client = TcpClient::new(config);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_not_connected_initially() {
        let client = TcpClient::new(TcpClientConfig::default());
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_send_without_connect() {
        let mut client = TcpClient::new(TcpClientConfig::default());

        let device_id = DeviceId::new(1).unwrap();
        let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();

        let result = client.send(msg).await;
        assert!(matches!(result, Err(TcpClientError::NotConnected)));
    }

    #[tokio::test]
    async fn test_recv_without_connect() {
        let mut client = TcpClient::new(TcpClientConfig::default());

        let result = client.recv().await;
        assert!(matches!(result, Err(TcpClientError::NotConnected)));
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        // Use a non-routable IP address (RFC 5737 TEST-NET-1)
        let config = TcpClientConfig {
            server_addr: "192.0.2.1:9999".parse().unwrap(),
            timeout: Duration::from_millis(100),
        };

        let mut client = TcpClient::new(config);
        let result = client.connect().await;

        assert!(matches!(result, Err(TcpClientError::ConnectionTimeout(_))));
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_close_when_not_connected() {
        let mut client = TcpClient::new(TcpClientConfig::default());

        // Should not error
        let result = client.close().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_close_calls() {
        let mut client = TcpClient::new(TcpClientConfig::default());

        // Multiple close calls should be safe
        client.close().await.unwrap();
        client.close().await.unwrap();
        client.close().await.unwrap();
    }
}
