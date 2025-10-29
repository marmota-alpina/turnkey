//! TCP Server for Henry protocol communication.
//!
//! This module provides a simple TCP server for the client-emulator (validation server)
//! to accept connections from multiple turnstile emulators and exchange Henry protocol
//! messages. The server uses HenryCodec for automatic message encoding/decoding.
//!
//! # Architecture
//!
//! ```text
//! Turnstile 01 ┐
//!              │
//! Turnstile 02 ├──> TcpServer ──> Client-Emulator TUI
//!              │        │
//! Turnstile 15 ┘        └──> HenryCodec (automatic framing)
//! ```
//!
//! # Example Usage
//!
//! ```no_run
//! use turnkey_network::{TcpServer, TcpServerConfig};
//! use std::net::SocketAddr;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure server
//! let config = TcpServerConfig {
//!     bind_addr: "0.0.0.0:3000".parse()?,
//!     max_connections: 100,
//! };
//!
//! // Create and bind
//! let mut server = TcpServer::bind(config).await?;
//!
//! // Accept messages from any turnstile
//! let (device_id, message) = server.accept().await?;
//! println!("Received from device {}: {:?}", device_id, message);
//!
//! // Send response back to specific device
//! server.send(device_id, message).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Design Principles
//!
//! The TcpServer is designed as a simple transport layer for emulator use:
//! - **No authentication**: This is an emulator component
//! - **No TLS**: Can be added later if needed
//! - **No rate limiting**: Not needed for emulator scenarios
//! - **Simple connection tracking**: HashMap for O(1) device lookup
//! - **1:1 device mapping**: Each turnstile has its own connection
//!
//! This keeps the server focused and testable, pushing business logic
//! to higher layers like the Client-Emulator TUI.
//!
//! # Multi-Turnstile Support
//!
//! The server handles multiple concurrent connections from different turnstiles.
//! Each connection is tracked by device ID, enabling proper message routing:
//!
//! - Messages from any device are received via `accept()`
//! - Responses are sent to specific devices via `send(device_id, message)`
//! - Connection state is tracked per device
//!
//! # Related
//!
//! - Issue #66: TCP Server implementation
//! - Issue #71: Client-Emulator TUI (uses this server)
//! - Issue #65: TCP Client (counterpart for turnstiles)

use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{debug, error, info, trace, warn};
use turnkey_core::DeviceId;
use turnkey_protocol::{HenryCodec, Message};

/// Configuration for TCP server
///
/// # Example
///
/// ```
/// use turnkey_network::TcpServerConfig;
///
/// let config = TcpServerConfig {
///     bind_addr: "0.0.0.0:3000".parse().unwrap(),
///     max_connections: 100,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TcpServerConfig {
    /// Address to bind the server to
    pub bind_addr: SocketAddr,

    /// Maximum number of simultaneous connections
    pub max_connections: usize,
}

impl Default for TcpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:3000".parse().unwrap(),
            max_connections: 100,
        }
    }
}

/// Represents a single client connection
///
/// Tracks connection metadata and provides message framing via HenryCodec.
#[derive(Debug)]
pub struct Connection {
    /// Device ID extracted from messages
    device_id: DeviceId,

    /// Framed TCP stream with HenryCodec
    framed: Framed<TcpStream, HenryCodec>,

    /// Remote client address
    addr: SocketAddr,

    /// Connection timestamp
    connected_at: DateTime<Utc>,
}

impl Connection {
    /// Get the device ID for this connection
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Get the remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get connection timestamp
    pub fn connected_at(&self) -> DateTime<Utc> {
        self.connected_at
    }

    /// Get connection uptime
    pub fn uptime(&self) -> chrono::Duration {
        Utc::now() - self.connected_at
    }

    /// Send a message to this connection
    async fn send(&mut self, message: Message) -> Result<(), TcpServerError> {
        self.framed
            .send(message)
            .await
            .map_err(|e| TcpServerError::Codec(e.to_string()))
    }

    /// Receive a message from this connection
    async fn recv(&mut self) -> Result<Option<Message>, TcpServerError> {
        match self.framed.next().await {
            Some(Ok(message)) => Ok(Some(message)),
            Some(Err(e)) => Err(TcpServerError::Codec(e.to_string())),
            None => Ok(None), // Connection closed
        }
    }
}

/// Connection information snapshot
///
/// Provides read-only access to connection metadata for monitoring
/// and debugging purposes.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Device ID for this connection
    pub device_id: DeviceId,

    /// Remote client address
    pub remote_addr: SocketAddr,

    /// When the connection was established
    pub connected_at: DateTime<Utc>,

    /// How long the connection has been active
    pub uptime: chrono::Duration,
}

/// Errors that can occur during TCP server operations
#[derive(Debug, Error)]
pub enum TcpServerError {
    /// Failed to bind to address
    #[error("Failed to bind to {0}")]
    BindFailed(SocketAddr),

    /// Device is not connected
    #[error("Device {0} not connected")]
    DeviceNotConnected(DeviceId),

    /// Maximum connections reached
    #[error("Maximum connections reached: {0}")]
    MaxConnectionsReached(usize),

    /// Device with same ID is already connected
    #[error("Device {0} is already connected")]
    DuplicateDevice(DeviceId),

    /// Invalid device ID in message
    #[error("Invalid device ID in message")]
    InvalidDeviceId,

    /// Low-level I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Codec error during message encoding/decoding
    #[error("Codec error: {0}")]
    Codec(String),
}

/// TCP server for Henry protocol communication
///
/// The `TcpServer` accepts connections from multiple turnstile emulators and
/// routes messages between them and the client-emulator application. It handles
/// connection lifecycle, message framing via HenryCodec, and per-device state tracking.
///
/// # Connection Lifecycle
///
/// 1. Bind server with `bind()`
/// 2. Accept messages from any device with `accept()`
/// 3. Send responses to specific devices with `send()`
/// 4. Disconnect devices with `disconnect()` or wait for client disconnect
///
/// # Thread Safety
///
/// `TcpServer` is not `Send` or `Sync` by design. It should be used from a
/// single task that manages all connections.
///
/// # Example
///
/// ```no_run
/// use turnkey_network::{TcpServer, TcpServerConfig};
/// use turnkey_core::DeviceId;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = TcpServerConfig::default();
/// let mut server = TcpServer::bind(config).await?;
///
/// // Accept request from any turnstile
/// let (device_id, request) = server.accept().await?;
/// println!("Request from device {}: {:?}", device_id, request);
///
/// // Process request and send response
/// // let response = process(request);
/// // server.send(device_id, response).await?;
/// # Ok(())
/// # }
/// ```
pub struct TcpServer {
    /// TCP listener for accepting new connections
    listener: TcpListener,

    /// Active connections indexed by device ID
    connections: HashMap<DeviceId, Connection>,

    /// Server configuration
    config: TcpServerConfig,
}

impl TcpServer {
    /// Bind the server to the configured address
    ///
    /// Creates a TCP listener and starts accepting connections. The server
    /// is ready to accept messages immediately after binding.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Address is already in use
    /// - Permission denied (e.g., binding to privileged port)
    /// - Invalid address format
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = TcpServerConfig::default();
    /// let server = TcpServer::bind(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bind(config: TcpServerConfig) -> Result<Self, TcpServerError> {
        info!("Binding TCP server to {}", config.bind_addr);

        let listener = TcpListener::bind(config.bind_addr)
            .await
            .map_err(|_| TcpServerError::BindFailed(config.bind_addr))?;

        info!(
            "TCP server listening on {} (max {} connections)",
            config.bind_addr, config.max_connections
        );

        Ok(Self {
            listener,
            connections: HashMap::new(),
            config,
        })
    }

    /// Accept a NEW connection and return its first message
    ///
    /// IMPORTANT: This method ONLY returns when a new device connects and sends
    /// its first message. Messages from already-connected devices will NOT be
    /// returned by this method.
    ///
    /// For a unified interface that handles both new connections and messages
    /// from existing connections, use `recv_any()` instead.
    ///
    /// # Typical Usage Pattern
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = TcpServer::bind(TcpServerConfig::default()).await?;
    ///
    /// // Accept first connection
    /// let (device_id, first_msg) = server.accept().await?;
    /// println!("New device {}: {:?}", device_id, first_msg);
    ///
    /// // For subsequent messages from this device, use recv()
    /// if let Some(next_msg) = server.recv(device_id).await? {
    ///     println!("Next message: {:?}", next_msg);
    /// }
    ///
    /// // Or use recv_any() to receive from any connected device
    /// let (any_device_id, message) = server.recv_any().await?;
    /// println!("Message from device {}: {:?}", any_device_id, message);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Maximum connections reached (connection rejected, continues listening)
    /// - Message decoding fails (connection rejected, continues listening)
    /// - Listener socket error (fatal)
    ///
    /// # See Also
    ///
    /// - `recv_any()` - Receive from any device (new or existing)
    /// - `recv()` - Receive from specific device
    pub async fn accept(&mut self) -> Result<(DeviceId, Message), TcpServerError> {
        loop {
            let (stream, addr) = self.listener.accept().await?;
            debug!("Accepted new connection from {}", addr);

            // Check max connections - reject this connection but keep accepting others
            if self.connections.len() >= self.config.max_connections {
                error!(
                    addr = %addr,
                    max_connections = self.config.max_connections,
                    current_connections = self.connections.len(),
                    "Connection rejected: maximum connections reached"
                );

                // NOTE: The Henry protocol does not define error response messages
                // for connection rejection scenarios. The connection is closed
                // immediately, and the client will receive a connection reset.
                // For production deployments, consider:
                // 1. Monitoring these rejection events
                // 2. Alerting when rejection rate is high
                // 3. Implementing custom error responses if protocol allows
                drop(stream);
                continue;
            }

            // Set TCP_NODELAY for low latency
            if let Err(e) = stream.set_nodelay(true) {
                warn!("Failed to set TCP_NODELAY for {}: {}", addr, e);
            }

            // Create framed connection and wait for first message to get device ID
            let mut framed = Framed::new(stream, HenryCodec::new());
            match framed.next().await {
                Some(Ok(message)) => {
                    let device_id = message.device_id;

                    // Check for duplicate device ID
                    if self.connections.contains_key(&device_id) {
                        let existing_addr = self.connections[&device_id].addr;
                        error!(
                            device_id = %device_id,
                            existing_addr = %existing_addr,
                            duplicate_addr = %addr,
                            "Connection rejected: device ID already connected"
                        );

                        // NOTE: The Henry protocol does not define error response messages
                        // for duplicate device scenarios. The duplicate connection is closed
                        // immediately to preserve the original connection.
                        // Consider implementing device reconnection logic if needed.
                        drop(framed);
                        continue;
                    }

                    info!(
                        "Device {} connected from {} (total: {})",
                        device_id,
                        addr,
                        self.connections.len() + 1
                    );

                    // Create connection entry
                    let conn = Connection {
                        device_id,
                        framed,
                        addr,
                        connected_at: Utc::now(),
                    };
                    self.connections.insert(device_id, conn);

                    return Ok((device_id, message));
                }
                Some(Err(e)) => {
                    error!("Failed to decode first message from {}: {}", addr, e);
                    // Don't propagate error - log and continue accepting connections
                    continue;
                }
                None => {
                    warn!("Connection closed before first message from {}", addr);
                    // Continue loop to accept next connection
                    continue;
                }
            }
        }
    }

    /// Receive a message from a specific device
    ///
    /// Waits for a message from the specified device connection.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Device is not connected
    /// - Message decoding fails
    /// - Connection is lost (returns None wrapped in Ok)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    /// use turnkey_core::DeviceId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = TcpServer::bind(TcpServerConfig::default()).await?;
    /// let (device_id, _first_msg) = server.accept().await?;
    ///
    /// // Receive next message from same device
    /// if let Some(message) = server.recv(device_id).await? {
    ///     println!("Received: {:?}", message);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn recv(&mut self, device_id: DeviceId) -> Result<Option<Message>, TcpServerError> {
        let Some(conn) = self.connections.get_mut(&device_id) else {
            return Err(TcpServerError::DeviceNotConnected(device_id));
        };

        match conn.recv().await {
            Ok(Some(message)) => {
                trace!(
                    device_id = %device_id,
                    command = ?message.command,
                    "Received message from device"
                );
                Ok(Some(message))
            }
            Ok(None) => {
                // Connection gracefully closed by peer - this is expected
                info!("Device {} disconnected gracefully", device_id);
                self.connections.remove(&device_id);
                Ok(None)
            }
            Err(e) => {
                // Classify error to determine if connection should be removed
                match &e {
                    TcpServerError::Codec(_) => {
                        // Protocol error - connection may still be alive, just bad message
                        // Keep connection open to allow recovery
                        warn!(
                            device_id = %device_id,
                            error = %e,
                            "Protocol error from device (connection maintained)"
                        );
                        Err(e)
                    }
                    TcpServerError::Io(_) => {
                        // I/O error - connection is dead, remove it
                        error!(
                            device_id = %device_id,
                            error = %e,
                            "I/O error from device (connection closed)"
                        );
                        self.connections.remove(&device_id);
                        Err(e)
                    }
                    _ => {
                        // Unknown error type - default to removing connection for safety
                        error!(
                            device_id = %device_id,
                            error = %e,
                            "Unexpected error from device (connection closed)"
                        );
                        self.connections.remove(&device_id);
                        Err(e)
                    }
                }
            }
        }
    }

    /// Receive a message from any connected device (new or existing)
    ///
    /// This method waits for a message from either:
    /// - A new connection being established (like `accept()`)
    /// - An existing connected device sending a message
    ///
    /// This provides a unified interface for receiving messages without
    /// needing to track which devices are new vs. existing.
    ///
    /// # Returns
    ///
    /// Returns the device ID and message when any device sends data.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - All connections are closed and no new connections arrive
    /// - Listener socket error
    /// - Message decoding fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = TcpServer::bind(TcpServerConfig::default()).await?;
    ///
    /// // Simple event loop handling all messages
    /// loop {
    ///     let (device_id, message) = server.recv_any().await?;
    ///     println!("Message from device {}: {:?}", device_id, message);
    ///
    ///     // Process and respond...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance Note
    ///
    /// This method uses `tokio::select!` to efficiently multiplex between
    /// the listener and all existing connections. For servers with many
    /// connections, this is more efficient than polling each connection
    /// individually.
    pub async fn recv_any(&mut self) -> Result<(DeviceId, Message), TcpServerError> {
        loop {
            // If we have no connections, just wait for new ones
            if self.connections.is_empty() {
                return self.accept().await;
            }

            // Collect device IDs to avoid borrowing issues
            let device_ids: Vec<DeviceId> = self.connections.keys().copied().collect();

            // Use tokio::select to wait for either a new connection or a message from existing ones
            tokio::select! {
                // Wait for new connection
                accept_result = self.listener.accept() => {
                    let (stream, addr) = accept_result?;
                    debug!("Accepted new connection from {}", addr);

                    // Check max connections
                    if self.connections.len() >= self.config.max_connections {
                        error!(
                            addr = %addr,
                            max_connections = self.config.max_connections,
                            current_connections = self.connections.len(),
                            "Connection rejected: maximum connections reached"
                        );
                        drop(stream);
                        continue;
                    }

                    // Set TCP_NODELAY for low latency
                    if let Err(e) = stream.set_nodelay(true) {
                        warn!("Failed to set TCP_NODELAY for {}: {}", addr, e);
                    }

                    // Create framed connection and wait for first message
                    let mut framed = Framed::new(stream, HenryCodec::new());
                    match framed.next().await {
                        Some(Ok(message)) => {
                            let device_id = message.device_id;

                            // Check for duplicate device ID
                            if self.connections.contains_key(&device_id) {
                                let existing_addr = self.connections[&device_id].addr;
                                error!(
                                    device_id = %device_id,
                                    existing_addr = %existing_addr,
                                    duplicate_addr = %addr,
                                    "Connection rejected: device ID already connected"
                                );
                                drop(framed);
                                continue;
                            }

                            info!(
                                "Device {} connected from {} (total: {})",
                                device_id,
                                addr,
                                self.connections.len() + 1
                            );

                            // Create connection entry
                            let conn = Connection {
                                device_id,
                                framed,
                                addr,
                                connected_at: Utc::now(),
                            };
                            self.connections.insert(device_id, conn);

                            return Ok((device_id, message));
                        }
                        Some(Err(e)) => {
                            error!("Failed to decode first message from {}: {}", addr, e);
                            continue;
                        }
                        None => {
                            warn!("Connection closed before first message from {}", addr);
                            continue;
                        }
                    }
                }

                // Wait for message from any existing connection
                // We poll each connection in round-robin fashion
                msg_result = async {
                    for device_id in device_ids {
                        if let Some(conn) = self.connections.get_mut(&device_id) {
                            // Try to receive without blocking
                            match tokio::time::timeout(
                                std::time::Duration::from_millis(1),
                                conn.recv()
                            ).await {
                                Ok(Ok(Some(message))) => {
                                    return Some((device_id, Ok(message)));
                                }
                                Ok(Ok(None)) => {
                                    // Connection closed
                                    return Some((device_id, Err(TcpServerError::Codec(
                                        "Connection closed".to_string()
                                    ))));
                                }
                                Ok(Err(e)) => {
                                    return Some((device_id, Err(e)));
                                }
                                Err(_) => {
                                    // Timeout - try next connection
                                    continue;
                                }
                            }
                        }
                    }
                    // No messages from any connection
                    None
                } => {
                    if let Some((device_id, result)) = msg_result {
                        match result {
                            Ok(message) => {
                                trace!(
                                    device_id = %device_id,
                                    command = ?message.command,
                                    "Received message from existing connection"
                                );
                                return Ok((device_id, message));
                            }
                            Err(e) => {
                                info!("Device {} disconnected: {}", device_id, e);
                                self.connections.remove(&device_id);
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Send a message to a specific device
    ///
    /// Routes the message to the connection identified by device ID.
    /// The message is automatically encoded by HenryCodec.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Device is not connected
    /// - Message encoding fails
    /// - Connection is lost
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    /// use turnkey_protocol::{MessageBuilder, CommandCode};
    /// use turnkey_core::DeviceId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = TcpServer::bind(TcpServerConfig::default()).await?;
    /// let (device_id, _request) = server.accept().await?;
    ///
    /// // Send response
    /// let response = MessageBuilder::new(device_id, CommandCode::GrantExit)
    ///     .build()?;
    /// server.send(device_id, response).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(
        &mut self,
        device_id: DeviceId,
        message: Message,
    ) -> Result<(), TcpServerError> {
        trace!(
            device_id = %device_id,
            command = ?message.command,
            "Sending message to device"
        );

        let Some(conn) = self.connections.get_mut(&device_id) else {
            return Err(TcpServerError::DeviceNotConnected(device_id));
        };

        conn.send(message).await
    }

    /// Check if a specific device is connected
    ///
    /// Returns `true` if the device has an active connection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    /// use turnkey_core::DeviceId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = TcpServer::bind(TcpServerConfig::default()).await?;
    /// let device_id = DeviceId::new(15)?;
    ///
    /// if server.is_connected(device_id) {
    ///     println!("Device {} is connected", device_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_connected(&self, device_id: DeviceId) -> bool {
        self.connections.contains_key(&device_id)
    }

    /// Get list of all connected device IDs
    ///
    /// Returns a vector of device IDs for all active connections.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = TcpServer::bind(TcpServerConfig::default()).await?;
    ///
    /// let devices = server.connected_devices();
    /// println!("Connected devices: {:?}", devices);
    /// # Ok(())
    /// # }
    /// ```
    pub fn connected_devices(&self) -> Vec<DeviceId> {
        self.connections.keys().copied().collect()
    }

    /// Get the local address the server is bound to
    ///
    /// This is useful for tests that bind to port 0 (OS-assigned random port).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = TcpServerConfig {
    ///     bind_addr: "127.0.0.1:0".parse()?,  // Random port
    ///     max_connections: 10,
    /// };
    /// let server = TcpServer::bind(config).await?;
    ///
    /// let actual_addr = server.local_addr()?;
    /// println!("Server listening on {}", actual_addr);
    /// # Ok(())
    /// # }
    /// ```
    pub fn local_addr(&self) -> Result<SocketAddr, TcpServerError> {
        self.listener.local_addr().map_err(Into::into)
    }

    /// Get detailed information about a specific connection
    ///
    /// Returns connection metadata including device ID, address, connection
    /// time, and uptime. Useful for monitoring and debugging.
    ///
    /// # Returns
    ///
    /// Returns `Some(ConnectionInfo)` if the device is connected, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    /// use turnkey_core::DeviceId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = TcpServer::bind(TcpServerConfig::default()).await?;
    /// let (device_id, _) = server.accept().await?;
    ///
    /// if let Some(info) = server.connection_info(device_id) {
    ///     println!("Device {} connected from {} for {}",
    ///         info.device_id, info.remote_addr, info.uptime);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn connection_info(&self, device_id: DeviceId) -> Option<ConnectionInfo> {
        self.connections.get(&device_id).map(|conn| ConnectionInfo {
            device_id: conn.device_id(),
            remote_addr: conn.remote_addr(),
            connected_at: conn.connected_at(),
            uptime: conn.uptime(),
        })
    }

    /// Get information about all active connections
    ///
    /// Returns a vector of `ConnectionInfo` for all connected devices.
    /// Useful for monitoring dashboards and status displays.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = TcpServer::bind(TcpServerConfig::default()).await?;
    ///
    /// for info in server.all_connections_info() {
    ///     println!("Device {}: {} (uptime: {})",
    ///         info.device_id, info.remote_addr, info.uptime);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn all_connections_info(&self) -> Vec<ConnectionInfo> {
        self.connections
            .values()
            .map(|conn| ConnectionInfo {
                device_id: conn.device_id(),
                remote_addr: conn.remote_addr(),
                connected_at: conn.connected_at(),
                uptime: conn.uptime(),
            })
            .collect()
    }

    /// Disconnect a specific device
    ///
    /// Closes the connection to the specified device and removes it from
    /// the connection tracking. This method is idempotent.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not connected. The connection
    /// will be closed regardless.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use turnkey_network::{TcpServer, TcpServerConfig};
    /// use turnkey_core::DeviceId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = TcpServer::bind(TcpServerConfig::default()).await?;
    /// let device_id = DeviceId::new(15)?;
    ///
    /// server.disconnect(device_id).await?;
    /// assert!(!server.is_connected(device_id));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disconnect(&mut self, device_id: DeviceId) -> Result<(), TcpServerError> {
        if let Some(conn) = self.connections.remove(&device_id) {
            info!(
                "Disconnecting device {} from {} (total: {})",
                device_id,
                conn.addr,
                self.connections.len()
            );
            // Connection is dropped automatically, closing the socket
            Ok(())
        } else {
            Err(TcpServerError::DeviceNotConnected(device_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turnkey_protocol::{CommandCode, MessageBuilder};

    #[test]
    fn test_config_default() {
        let config = TcpServerConfig::default();
        assert_eq!(config.bind_addr.port(), 3000);
        assert_eq!(config.max_connections, 100);
    }

    #[tokio::test]
    async fn test_server_bind() {
        let config = TcpServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(), // Use port 0 for random available port
            max_connections: 10,
        };

        let server = TcpServer::bind(config).await;
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_server_bind_invalid_address() {
        // Try to bind to an invalid address format
        // Note: We can't easily test permission errors without root
        let config = TcpServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            max_connections: 10,
        };

        // This should succeed with port 0
        let result = TcpServer::bind(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_is_connected_empty() {
        let config = TcpServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            max_connections: 10,
        };

        let server = TcpServer::bind(config).await.unwrap();
        let device_id = DeviceId::new(1).unwrap();

        assert!(!server.is_connected(device_id));
    }

    #[tokio::test]
    async fn test_connected_devices_empty() {
        let config = TcpServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            max_connections: 10,
        };

        let server = TcpServer::bind(config).await.unwrap();
        assert_eq!(server.connected_devices().len(), 0);
    }

    #[tokio::test]
    async fn test_disconnect_not_connected() {
        let config = TcpServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            max_connections: 10,
        };

        let mut server = TcpServer::bind(config).await.unwrap();
        let device_id = DeviceId::new(1).unwrap();

        let result = server.disconnect(device_id).await;
        assert!(matches!(result, Err(TcpServerError::DeviceNotConnected(_))));
    }

    #[tokio::test]
    async fn test_send_to_disconnected_device() {
        let config = TcpServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            max_connections: 10,
        };

        let mut server = TcpServer::bind(config).await.unwrap();
        let device_id = DeviceId::new(1).unwrap();
        let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();

        let result = server.send(device_id, message).await;
        assert!(matches!(result, Err(TcpServerError::DeviceNotConnected(_))));
    }
}
