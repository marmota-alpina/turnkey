# Issue #65: TCP Client for Turnstile Emulator - Technical Specification

**Status:** OPEN
**Created:** 2025-10-26
**Updated:** 2025-10-27
**Author:** marmota-alpina
**Labels:** MVP, network, phase-2

## Overview

Implement a simple TCP client for the turnstile emulator to connect to the client-emulator (validation server) and exchange Henry protocol messages. This is a basic transport layer component used by the OnlineValidator.

**Key Principle:** This is an emulator component, not a production system. Focus on simplicity and correctness over performance and resilience.

## Architecture

### Component Diagram

```
┌─────────────────────────────────────┐
│    TurnstileEmulator (Issue #73)    │
│                                     │
│  ┌───────────────────────────────┐  │
│  │  OnlineValidator (Issue #69)  │  │
│  │         │                     │  │
│  │         ▼                     │  │
│  │  ┌─────────────────┐          │  │
│  │  │   TcpClient     │◄─────────┼──┼─── Issue #65 (This)
│  │  │  (Issue #65)    │          │  │
│  │  └────────┬────────┘          │  │
│  │           │                   │  │
│  └───────────┼───────────────────┘  │
│              │ Henry Protocol       │
│              ▼                      │
└──────────────┼──────────────────────┘
               │
        TCP Connection
               │
               ▼
┌──────────────┴──────────────────────┐
│   Client Emulator (Issue #66)       │
│   TCP Server (validation server)    │
└─────────────────────────────────────┘
```

### Responsibility

**TcpClient** is responsible for:
- Establishing TCP connection to validation server
- Encoding messages using HenryCodec
- Sending AccessRequest messages
- Receiving AccessResponse messages
- Basic timeout handling
- Clean connection closure

**TcpClient is NOT responsible for:**
- Business logic (handled by OnlineValidator)
- Retry logic (caller decides when to retry)
- Message validation (handled by protocol layer)
- Connection pooling (single connection per turnstile)
- Keepalive (connections are short-lived)

## API Design

### Struct Definition

```rust
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use turnkey_protocol::{HenryCodec, Message};
use std::net::SocketAddr;
use std::time::Duration;

pub struct TcpClient {
    /// Server address to connect to
    server_addr: SocketAddr,

    /// Framed stream with HenryCodec (None if not connected)
    framed: Option<Framed<TcpStream, HenryCodec>>,

    /// Default timeout for operations
    timeout: Duration,
}

pub struct TcpClientConfig {
    pub server_addr: SocketAddr,
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
```

### Public Methods

```rust
impl TcpClient {
    /// Create a new TCP client with configuration
    ///
    /// # Example
    /// ```no_run
    /// use turnkey_network::{TcpClient, TcpClientConfig};
    /// use std::net::SocketAddr;
    /// use std::time::Duration;
    ///
    /// let config = TcpClientConfig {
    ///     server_addr: "192.168.0.100:3000".parse().unwrap(),
    ///     timeout: Duration::from_millis(5000),
    /// };
    /// let client = TcpClient::new(config);
    /// ```
    pub fn new(config: TcpClientConfig) -> Self;

    /// Connect to the validation server
    ///
    /// # Errors
    /// - Connection timeout
    /// - Connection refused
    /// - Invalid address
    ///
    /// # Example
    /// ```no_run
    /// # use turnkey_network::{TcpClient, TcpClientConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&mut self) -> Result<()>;

    /// Send a message to the server
    ///
    /// # Errors
    /// - Not connected
    /// - Write timeout
    /// - Connection lost
    ///
    /// # Example
    /// ```no_run
    /// # use turnkey_network::{TcpClient, TcpClientConfig};
    /// # use turnkey_protocol::{Message, CommandCode, MessageBuilder};
    /// # use turnkey_core::DeviceId;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    ///
    /// let device_id = DeviceId::new(15)?;
    /// let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
    ///     .with_field("1234567890")?
    ///     .with_field("27/10/2025 14:30:00")?
    ///     .build()?;
    ///
    /// client.send(msg).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(&mut self, message: Message) -> Result<()>;

    /// Receive a message from the server with timeout
    ///
    /// # Errors
    /// - Not connected
    /// - Read timeout
    /// - Connection lost
    /// - Invalid message format
    ///
    /// # Example
    /// ```no_run
    /// # use turnkey_network::{TcpClient, TcpClientConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    ///
    /// let response = client.recv().await?;
    /// println!("Received: {:?}", response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn recv(&mut self) -> Result<Message>;

    /// Check if client is connected
    pub fn is_connected(&self) -> bool;

    /// Close the connection gracefully
    ///
    /// # Example
    /// ```no_run
    /// # use turnkey_network::{TcpClient, TcpClientConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = TcpClient::new(TcpClientConfig::default());
    /// client.connect().await?;
    /// // ... use client ...
    /// client.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(&mut self) -> Result<()>;
}
```

## Error Handling

### Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TcpClientError {
    #[error("Not connected to server")]
    NotConnected,

    #[error("Connection timeout after {0:?}")]
    ConnectionTimeout(Duration),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Read timeout after {0:?}")]
    ReadTimeout(Duration),

    #[error("Write timeout after {0:?}")]
    WriteTimeout(Duration),

    #[error("Connection lost: {0}")]
    ConnectionLost(String),

    #[error("Protocol error: {0}")]
    ProtocolError(#[from] turnkey_protocol::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### Error Handling Strategy

**Connection Errors:**
- Caller (OnlineValidator) decides whether to retry
- No automatic reconnection
- Clear error messages for debugging

**Timeout Errors:**
- Use tokio::time::timeout for all I/O operations
- Configurable timeout (default: 3000ms)
- Return timeout error to caller

**Protocol Errors:**
- Forward protocol errors from HenryCodec
- Let protocol layer handle validation
- No recovery at TCP layer

## Implementation Details

### Connection Flow

```rust
// Simplified implementation
pub async fn connect(&mut self) -> Result<()> {
    // 1. Connect with timeout
    let stream = tokio::time::timeout(
        self.timeout,
        TcpStream::connect(self.server_addr)
    ).await??;

    // 2. Configure TCP options
    stream.set_nodelay(true)?; // Disable Nagle for low latency

    // 3. Wrap with HenryCodec
    self.framed = Some(Framed::new(stream, HenryCodec::new()));

    Ok(())
}
```

### Send Message Flow

```rust
use futures::SinkExt;

pub async fn send(&mut self, message: Message) -> Result<()> {
    // 1. Check connected
    let framed = self.framed.as_mut()
        .ok_or(TcpClientError::NotConnected)?;

    // 2. Send with timeout
    tokio::time::timeout(
        self.timeout,
        framed.send(message)
    ).await??;

    Ok(())
}
```

### Receive Message Flow

```rust
use futures::StreamExt;

pub async fn recv(&mut self) -> Result<Message> {
    // 1. Check connected
    let framed = self.framed.as_mut()
        .ok_or(TcpClientError::NotConnected)?;

    // 2. Receive with timeout
    let message = tokio::time::timeout(
        self.timeout,
        framed.next()
    ).await?
        .ok_or(TcpClientError::ConnectionLost("Stream ended".into()))?
        .map_err(TcpClientError::ProtocolError)?;

    Ok(message)
}
```

## Integration with OnlineValidator

### Usage Example

```rust
use turnkey_network::{TcpClient, TcpClientConfig};
use turnkey_protocol::commands::access::{AccessRequest, AccessResponse};

pub struct OnlineValidator {
    tcp_client: TcpClient,
    device_id: DeviceId,
}

impl OnlineValidator {
    pub async fn new(server_addr: SocketAddr, device_id: DeviceId) -> Result<Self> {
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(3000),
        };

        let mut tcp_client = TcpClient::new(config);
        tcp_client.connect().await?;

        Ok(Self { tcp_client, device_id })
    }

    pub async fn validate(&mut self, request: &AccessRequest) -> Result<AccessResponse> {
        // 1. Convert AccessRequest to Message
        let message = request.to_message(self.device_id)?;

        // 2. Send request
        self.tcp_client.send(message).await?;

        // 3. Receive response
        let response_msg = self.tcp_client.recv().await?;

        // 4. Parse AccessResponse
        AccessResponse::from_message(&response_msg)
    }
}
```

## Configuration

### Default Configuration

```toml
# config/default.toml
[network.client]
server_addr = "127.0.0.1:3000"
timeout_ms = 3000
tcp_nodelay = true
```

### Environment Overrides

```bash
# Override server address
export TURNKEY_SERVER_ADDR="192.168.0.100:3000"

# Override timeout
export TURNKEY_TIMEOUT_MS=5000
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = TcpClientConfig::default();
        let client = TcpClient::new(config);
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_connect_timeout() {
        let config = TcpClientConfig {
            server_addr: "192.0.2.1:9999".parse().unwrap(), // Non-routable IP
            timeout: Duration::from_millis(100),
        };

        let mut client = TcpClient::new(config);
        let result = client.connect().await;

        assert!(matches!(result, Err(TcpClientError::ConnectionTimeout(_))));
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
}
```

### Integration Tests

```rust
// tests/integration_tcp_client.rs
use turnkey_network::{TcpClient, TcpClientConfig, TcpServer};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_send_receive_flow() {
    // 1. Start mock server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream, HenryCodec::new());

        // Echo messages back
        while let Some(Ok(msg)) = framed.next().await {
            framed.send(msg).await.unwrap();
        }
    });

    // 2. Connect client
    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    // 3. Send message
    let device_id = DeviceId::new(15).unwrap();
    let sent_msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
        .build()
        .unwrap();

    client.send(sent_msg.clone()).await.unwrap();

    // 4. Receive echo
    let received_msg = client.recv().await.unwrap();

    assert_eq!(sent_msg.device_id, received_msg.device_id);
    assert_eq!(sent_msg.command_code, received_msg.command_code);

    // 5. Close
    client.close().await.unwrap();
}

#[tokio::test]
async fn test_recv_timeout() {
    // Start server that doesn't respond
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        // Don't send anything - just hold connection
        tokio::time::sleep(Duration::from_secs(10)).await;
    });

    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(100),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    let result = client.recv().await;
    assert!(matches!(result, Err(TcpClientError::ReadTimeout(_))));
}
```

## Dependencies

### Cargo.toml

```toml
[package]
name = "turnkey-network"
version = "0.1.0"
edition = "2024"

[dependencies]
# Async runtime
tokio = { workspace = true, features = ["net", "time", "io-util"] }
tokio-util = { version = "0.7", features = ["codec"] }

# Protocol
turnkey-protocol = { path = "../turnkey-protocol" }
turnkey-core = { path = "../turnkey-core" }

# Utilities
bytes = { workspace = true }
futures = "0.3"

# Error handling
thiserror = { workspace = true }

# Logging
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["full", "test-util"] }
```

## Acceptance Criteria

- [x] TcpClient struct implemented
- [x] Connect with timeout works
- [x] Send message works
- [x] Receive message works with timeout
- [x] Error handling complete
- [x] Close connection works
- [x] Unit tests pass
- [x] Integration tests with mock server pass
- [x] Documentation complete
- [x] Code formatted and clippy clean

## Implementation Checklist

### Phase 1: Basic Structure
- [ ] Create `crates/turnkey-network/src/client.rs`
- [ ] Define `TcpClient` struct
- [ ] Define `TcpClientConfig` struct
- [ ] Define `TcpClientError` enum
- [ ] Implement `new()` and `is_connected()`

### Phase 2: Connection Management
- [ ] Implement `connect()` with timeout
- [ ] Implement `close()`
- [ ] Add TCP options (nodelay)

### Phase 3: Message I/O
- [ ] Implement `send()` with timeout
- [ ] Implement `recv()` with timeout
- [ ] Handle connection loss

### Phase 4: Testing
- [ ] Add unit tests
- [ ] Add integration tests with mock server
- [ ] Test timeout scenarios
- [ ] Test error conditions

### Phase 5: Documentation
- [ ] Add rustdoc comments
- [ ] Add usage examples
- [ ] Update lib.rs exports

## Related Issues

- **Depends on:** Issue #5 (HenryCodec) ✅ DONE
- **Used by:** Issue #69 (OnlineValidator)
- **Counterpart:** Issue #66 (TCP Server)
- **Integration:** Issue #73 (TurnstileEmulator)

## Notes

### Why No Automatic Retry?

The TcpClient is a low-level transport component. Retry logic belongs in the OnlineValidator because:
- OnlineValidator knows when to retry (business logic)
- OnlineValidator can fallback to offline mode
- OnlineValidator can log retry attempts
- Simpler, more testable code

### Why No Connection Pooling?

Each turnstile emulator has a single connection to the validation server:
- Simpler lifecycle management
- Matches real hardware behavior
- No concurrency issues
- Adequate for emulator use case

### Why No Keepalive?

Connections are short-lived in the emulator:
- Connect → Send Request → Receive Response → Close
- Server detects disconnections naturally
- Simpler implementation
- Matches typical access control flow

## Future Enhancements (Not in Scope)

- TLS support (for production systems)
- Connection pooling (if needed)
- Automatic reconnection (if needed)
- Metrics collection (if needed)
- Health checks (if needed)

These can be added later if the project evolves beyond emulation.
