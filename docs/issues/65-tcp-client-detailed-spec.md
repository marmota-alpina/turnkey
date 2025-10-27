# Issue #65: TCP Client for Turnstile Emulator - Technical Specification

**Status:** OPEN
**Created:** 2025-10-26
**Updated:** 2025-10-27
**Author:** marmota-alpina
**Labels:** MVP, network, phase-2

## Overview

Implement a simple TCP client for the turnstile emulator to connect to the client-emulator (validation server) and exchange Henry protocol messages. This is a basic transport layer component used by the OnlineValidator.

**Key Principle:** This is an emulator component, not a production system. Focus on simplicity and correctness over performance and resilience.

## Complete Specification

For the complete detailed specification including:
- Architecture diagrams
- Complete API documentation with code examples
- Integration examples with OnlineValidator
- Comprehensive testing strategy
- Error handling details
- Implementation checklist

Please see the local documentation file:
`docs/issues/65-tcp-client-detailed-spec.md`

## Quick Reference

### API Overview

```rust
pub struct TcpClient {
    server_addr: SocketAddr,
    framed: Option<Framed<TcpStream, HenryCodec>>,
    timeout: Duration,
}

impl TcpClient {
    pub fn new(config: TcpClientConfig) -> Self;
    pub async fn connect(&mut self) -> Result<()>;
    pub async fn send(&mut self, message: Message) -> Result<()>;
    pub async fn recv(&mut self) -> Result<Message>;
    pub fn is_connected(&self) -> bool;
    pub async fn close(&mut self) -> Result<()>;
}
```

### Scope

**IN scope:**
- Basic TCP connection with timeout (default: 3000ms)
- Send/receive using HenryCodec
- Simple error handling
- Clean connection closure

**OUT of scope:**
- Automatic retry logic
- Keepalive mechanism
- Connection pooling
- Complex resilience patterns

### Dependencies

```toml
tokio = { workspace = true, features = ["net", "time", "io-util"] }
tokio-util = { version = "0.7", features = ["codec"] }
turnkey-protocol = { path = "../turnkey-protocol" }
futures = "0.3"
```

## Related Issues

- **Used by:** #69 (OnlineValidator)
- **Counterpart:** #66 (TCP Server)
- **Depends on:** #5 (HenryCodec) - ✅ DONE

## Documentation

See local file for complete specification:
`docs/issues/65-tcp-client-detailed-spec.md`