//! Integration tests for TcpClient
//!
//! These tests verify the complete connect-send-recv-close cycle with
//! a mock TCP server. They test real network I/O and timeout scenarios.

use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use turnkey_core::DeviceId;
use turnkey_network::{TcpClient, TcpClientConfig, TcpClientError};
use turnkey_protocol::{CommandCode, FieldData, HenryCodec, MessageBuilder};

/// Test basic connect-send-recv-close flow with echo server
#[tokio::test]
async fn test_full_lifecycle_with_echo_server() {
    // Start mock echo server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn echo server task
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream, HenryCodec::new());

        // Echo one message back
        if let Some(Ok(msg)) = framed.next().await {
            framed.send(msg).await.unwrap();
        }
    });

    // Create and connect client
    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    assert!(!client.is_connected());

    client.connect().await.unwrap();
    assert!(client.is_connected());

    // Send message
    let device_id = DeviceId::new(15).unwrap();
    let sent_msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
        .build()
        .unwrap();

    client.send(sent_msg.clone()).await.unwrap();

    // Receive echo
    let received_msg = client.recv().await.unwrap();

    assert_eq!(sent_msg.device_id, received_msg.device_id);
    assert_eq!(sent_msg.command, received_msg.command);

    // Close connection
    client.close().await.unwrap();
    assert!(!client.is_connected());
}

/// Test send and receive with complex message containing fields
#[tokio::test]
async fn test_send_recv_complex_message() {
    // Start mock server that echoes messages
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream, HenryCodec::new());

        while let Some(Ok(msg)) = framed.next().await {
            framed.send(msg).await.unwrap();
        }
    });

    // Connect client
    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    // Send complex message with fields
    let device_id = DeviceId::new(15).unwrap();
    let sent_msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("1234567890".to_string()).unwrap())
        .field(FieldData::new("27/10/2025 14:30:00".to_string()).unwrap())
        .field(FieldData::new("1".to_string()).unwrap())
        .field(FieldData::new("0".to_string()).unwrap())
        .build()
        .unwrap();

    client.send(sent_msg.clone()).await.unwrap();

    // Receive and verify
    let received_msg = client.recv().await.unwrap();

    assert_eq!(sent_msg.device_id, received_msg.device_id);
    assert_eq!(sent_msg.command, received_msg.command);
    assert_eq!(sent_msg.fields.len(), received_msg.fields.len());
    assert_eq!(sent_msg.field(0), received_msg.field(0));
    assert_eq!(sent_msg.field(1), received_msg.field(1));

    client.close().await.unwrap();
}

/// Test receiving timeout when server doesn't respond
#[tokio::test]
async fn test_recv_timeout() {
    // Start server that accepts but doesn't send anything
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        // Don't send anything - just hold connection open
        tokio::time::sleep(Duration::from_secs(10)).await;
    });

    // Connect client with short timeout
    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(100),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    // Try to receive - should timeout
    let result = client.recv().await;

    assert!(matches!(result, Err(TcpClientError::ReadTimeout(_))));

    if let Err(TcpClientError::ReadTimeout(ms)) = result {
        assert_eq!(ms, 100);
    }

    client.close().await.unwrap();
}

/// Test connection timeout with unreachable server
#[tokio::test]
async fn test_connection_timeout() {
    // Use TEST-NET-1 (RFC 5737) - should be unreachable
    let config = TcpClientConfig {
        server_addr: "192.0.2.1:9999".parse().unwrap(),
        timeout: Duration::from_millis(100),
    };

    let mut client = TcpClient::new(config);
    let result = client.connect().await;

    assert!(matches!(result, Err(TcpClientError::ConnectionTimeout(_))));

    if let Err(TcpClientError::ConnectionTimeout(ms)) = result {
        assert_eq!(ms, 100);
    }
}

/// Test connection refused error
#[tokio::test]
async fn test_connection_refused() {
    // Try to connect to a port that's not listening
    // Use a high port number that's unlikely to be in use
    let config = TcpClientConfig {
        server_addr: "127.0.0.1:55555".parse().unwrap(),
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    let result = client.connect().await;

    // Should fail (either connection refused or timeout)
    assert!(result.is_err());
}

/// Test server closes connection while client is receiving
#[tokio::test]
async fn test_connection_lost_during_recv() {
    // Start server that closes connection immediately after accept
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Close immediately
        drop(stream);
    });

    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    // Give server time to close
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Try to receive - should detect connection loss
    let result = client.recv().await;

    assert!(matches!(result, Err(TcpClientError::ConnectionLost(_))));
}

/// Test multiple sequential messages
#[tokio::test]
async fn test_multiple_messages() {
    // Start server that echoes multiple messages
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream, HenryCodec::new());

        while let Some(Ok(msg)) = framed.next().await {
            framed.send(msg).await.unwrap();
        }
    });

    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    // Send and receive multiple messages
    for i in 1..=5 {
        let device_id = DeviceId::new(i).unwrap();
        let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();

        client.send(msg.clone()).await.unwrap();
        let response = client.recv().await.unwrap();

        assert_eq!(msg.device_id, response.device_id);
    }

    client.close().await.unwrap();
}

/// Test that client can be reused after close
#[tokio::test]
async fn test_reconnect_after_close() {
    // Start server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let mut framed = Framed::new(stream, HenryCodec::new());
                while let Some(Ok(msg)) = framed.next().await {
                    framed.send(msg).await.unwrap();
                }
            }
        }
    });

    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);

    // First connection
    client.connect().await.unwrap();
    assert!(client.is_connected());

    let device_id = DeviceId::new(15).unwrap();
    let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
        .build()
        .unwrap();

    client.send(msg.clone()).await.unwrap();
    let _ = client.recv().await.unwrap();

    client.close().await.unwrap();
    assert!(!client.is_connected());

    // Second connection
    client.connect().await.unwrap();
    assert!(client.is_connected());

    client.send(msg.clone()).await.unwrap();
    let _ = client.recv().await.unwrap();

    client.close().await.unwrap();
}

/// Test handling of protocol errors (invalid messages)
#[tokio::test]
async fn test_protocol_error_handling() {
    // Start server that sends invalid data
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::AsyncWriteExt;
            // Send invalid Henry protocol data
            let _ = stream.write_all(b"\x02INVALID_DATA\x03").await;
        }
    });

    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    // Try to receive - should get protocol error
    let result = client.recv().await;

    assert!(matches!(result, Err(TcpClientError::Protocol(_))));
}

/// Test access request/response flow (realistic scenario)
#[tokio::test]
async fn test_access_request_response_flow() {
    // Start mock validation server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream, HenryCodec::new());

        // Wait for access request
        if let Some(Ok(request)) = framed.next().await {
            // Verify it's an access request
            assert_eq!(request.command, CommandCode::AccessRequest);

            // Send grant response
            let device_id = request.device_id;
            let response = MessageBuilder::new(device_id, CommandCode::GrantExit)
                .field(FieldData::new("5".to_string()).unwrap())
                .field(FieldData::new("Acesso liberado".to_string()).unwrap())
                .build()
                .unwrap();

            framed.send(response).await.unwrap();
        }
    });

    // Client sends access request
    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    let device_id = DeviceId::new(15).unwrap();
    let request = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("1234567890".to_string()).unwrap())
        .field(FieldData::new("27/10/2025 14:30:00".to_string()).unwrap())
        .field(FieldData::new("1".to_string()).unwrap())
        .field(FieldData::new("0".to_string()).unwrap())
        .build()
        .unwrap();

    client.send(request).await.unwrap();

    // Receive grant response
    let response = client.recv().await.unwrap();

    assert_eq!(response.device_id.as_u8(), 15);
    assert_eq!(response.command, CommandCode::GrantExit);
    assert_eq!(response.field(0), Some("5"));
    assert_eq!(response.field(1), Some("Acesso liberado"));

    client.close().await.unwrap();
}

/// Test handling of incomplete message (connection closes mid-message)
#[tokio::test]
async fn test_incomplete_message_handling() {
    // Start server that sends partial message then closes
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::AsyncWriteExt;
            // Send partial Henry protocol message then close connection mid-message
            // Format should be: \x02<ID>+REON+<COMMAND>+<FIELDS>\x03
            let _ = stream.write_all(b"\x0215+REON+").await;
            // Drop stream immediately (close mid-message)
            drop(stream);
        }
    });

    let config = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(1000),
    };

    let mut client = TcpClient::new(config);
    client.connect().await.unwrap();

    // Try to receive - should detect connection loss or protocol error
    let result = client.recv().await;

    // Should fail with either ConnectionLost or Protocol error
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(TcpClientError::ConnectionLost(_)) | Err(TcpClientError::Protocol(_))
    ));
}

/// Test custom timeout configuration
#[tokio::test]
async fn test_custom_timeout() {
    // Start server with intentional delay
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream, HenryCodec::new());

        if let Some(Ok(msg)) = framed.next().await {
            // Delay before responding
            tokio::time::sleep(Duration::from_millis(200)).await;
            framed.send(msg).await.unwrap();
        }
    });

    // Client with very short timeout - should fail
    let config_short = TcpClientConfig {
        server_addr: addr,
        timeout: Duration::from_millis(50),
    };

    let mut client = TcpClient::new(config_short);
    client.connect().await.unwrap();

    let device_id = DeviceId::new(1).unwrap();
    let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
        .build()
        .unwrap();

    client.send(msg.clone()).await.unwrap();

    let result = client.recv().await;
    assert!(matches!(result, Err(TcpClientError::ReadTimeout(_))));
}
