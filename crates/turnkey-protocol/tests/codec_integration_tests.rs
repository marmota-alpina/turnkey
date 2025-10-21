//! Integration tests for HenryCodec with Tokio streams.
//!
//! These tests verify the codec works correctly with real Tokio streams,
//! testing roundtrip encoding/decoding, partial message handling, and
//! error recovery scenarios.

use futures::{SinkExt, StreamExt};
use tokio::io::DuplexStream;
use tokio_util::codec::Framed;
use turnkey_core::DeviceId;
use turnkey_protocol::{CommandCode, FieldData, HenryCodec, MessageBuilder};

/// Helper function to create a framed duplex stream for testing.
fn create_framed_duplex(
    buffer_size: usize,
) -> (
    Framed<DuplexStream, HenryCodec>,
    Framed<DuplexStream, HenryCodec>,
) {
    let (client, server) = tokio::io::duplex(buffer_size);
    let client_framed = Framed::new(client, HenryCodec::new());
    let server_framed = Framed::new(server, HenryCodec::new());
    (client_framed, server_framed)
}

#[tokio::test]
async fn test_codec_roundtrip_simple_message() {
    let (mut client, mut server) = create_framed_duplex(1024);

    // Client sends query status
    let device_id = DeviceId::new(15).unwrap();
    let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
        .build()
        .unwrap();

    client.send(msg.clone()).await.unwrap();

    // Server receives the message
    let received = server.next().await.unwrap().unwrap();

    assert_eq!(received.device_id, msg.device_id);
    assert_eq!(received.command, msg.command);
    assert_eq!(received.fields.len(), msg.fields.len());
}

#[tokio::test]
async fn test_codec_roundtrip_message_with_fields() {
    let (mut client, mut server) = create_framed_duplex(1024);

    // Client sends access request with card data
    let device_id = DeviceId::new(15).unwrap();
    let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("12345678".to_string()).unwrap())
        .field(FieldData::new("10/05/2025 12:46:06".to_string()).unwrap())
        .field(FieldData::new("1".to_string()).unwrap())
        .field(FieldData::new("0".to_string()).unwrap())
        .build()
        .unwrap();

    client.send(msg.clone()).await.unwrap();

    // Server receives the message
    let received = server.next().await.unwrap().unwrap();

    assert_eq!(received.device_id, msg.device_id);
    assert_eq!(received.command, msg.command);
    assert_eq!(received.fields.len(), 4);
    assert_eq!(received.fields[0].as_str(), "12345678");
    assert_eq!(received.fields[1].as_str(), "10/05/2025 12:46:06");
}

#[tokio::test]
async fn test_codec_bidirectional_communication() {
    let (mut client, mut server) = create_framed_duplex(1024);

    let client_id = DeviceId::new(15).unwrap();
    let server_id = DeviceId::new(1).unwrap();

    // Client sends request
    let request = MessageBuilder::new(client_id, CommandCode::AccessRequest)
        .field(FieldData::new("12345678".to_string()).unwrap())
        .build()
        .unwrap();

    client.send(request.clone()).await.unwrap();

    // Server receives request
    let received_request = server.next().await.unwrap().unwrap();
    assert_eq!(received_request.device_id, client_id);

    // Server sends response
    let response = MessageBuilder::new(server_id, CommandCode::GrantEntry)
        .field(FieldData::new("5".to_string()).unwrap())
        .field(FieldData::new("Access granted".to_string()).unwrap())
        .build()
        .unwrap();

    server.send(response.clone()).await.unwrap();

    // Client receives response
    let received_response = client.next().await.unwrap().unwrap();
    assert_eq!(received_response.device_id, server_id);
    assert_eq!(received_response.command, CommandCode::GrantEntry);
}

#[tokio::test]
async fn test_codec_multiple_messages_in_sequence() {
    let (mut client, mut server) = create_framed_duplex(4096);

    let device_id = DeviceId::new(15).unwrap();

    // Send 10 messages in sequence
    for i in 0..10 {
        let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .field(FieldData::new(format!("message_{}", i)).unwrap())
            .build()
            .unwrap();

        client.send(msg).await.unwrap();
    }

    // Receive all 10 messages
    for i in 0..10 {
        let received = server.next().await.unwrap().unwrap();
        assert_eq!(received.device_id, device_id);
        assert_eq!(received.command, CommandCode::QueryStatus);
        assert_eq!(received.fields[0].as_str(), format!("message_{}", i));
    }
}

#[tokio::test]
async fn test_codec_with_small_buffer() {
    // Use relatively small buffer (but large enough to avoid deadlock)
    let (mut client, mut server) = create_framed_duplex(256);

    let device_id = DeviceId::new(15).unwrap();
    let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("12345678".to_string()).unwrap())
        .field(FieldData::new("10/05/2025 12:46:06".to_string()).unwrap())
        .build()
        .unwrap();

    client.send(msg.clone()).await.unwrap();

    let received = server.next().await.unwrap().unwrap();
    assert_eq!(received.device_id, msg.device_id);
    assert_eq!(received.command, msg.command);
}

#[tokio::test]
async fn test_codec_concurrent_sends() {
    let (mut client, mut server) = create_framed_duplex(4096);

    let device_id = DeviceId::new(15).unwrap();

    // Spawn task to send messages
    let send_task = tokio::spawn(async move {
        for i in 0..5 {
            let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
                .field(FieldData::new(format!("msg_{}", i)).unwrap())
                .build()
                .unwrap();

            client.send(msg).await.unwrap();
        }
    });

    // Receive messages
    let mut count = 0;
    while let Some(result) = server.next().await {
        let msg = result.unwrap();
        assert_eq!(msg.device_id, device_id);
        count += 1;

        if count >= 5 {
            break;
        }
    }

    send_task.await.unwrap();
    assert_eq!(count, 5);
}

#[tokio::test]
async fn test_codec_different_message_types() {
    let (mut client, mut server) = create_framed_duplex(4096);

    let device_id = DeviceId::new(15).unwrap();

    // Send different command types
    let commands = vec![
        CommandCode::QueryStatus,
        CommandCode::AccessRequest,
        CommandCode::GrantEntry,
        CommandCode::GrantExit,
        CommandCode::DenyAccess,
        CommandCode::SendConfig,
    ];

    for cmd in &commands {
        let msg = MessageBuilder::new(device_id, *cmd).build().unwrap();
        client.send(msg).await.unwrap();
    }

    // Receive and verify all commands
    for expected_cmd in &commands {
        let received = server.next().await.unwrap().unwrap();
        assert_eq!(received.command, *expected_cmd);
    }
}

#[tokio::test]
async fn test_codec_with_empty_fields() {
    let (mut client, mut server) = create_framed_duplex(1024);

    let device_id = DeviceId::new(15).unwrap();

    // Some Henry protocol messages have empty fields (represented by ]])
    let msg = MessageBuilder::new(device_id, CommandCode::WaitingRotation)
        .field(FieldData::new("".to_string()).unwrap())
        .field(FieldData::new("10/05/2025 12:46:06".to_string()).unwrap())
        .build()
        .unwrap();

    client.send(msg.clone()).await.unwrap();

    let received = server.next().await.unwrap().unwrap();
    assert_eq!(received.device_id, msg.device_id);
    assert_eq!(received.command, msg.command);
}

#[tokio::test]
async fn test_codec_max_frame_size_limit() {
    let (client, _server) = tokio::io::duplex(1024);
    let mut framed = Framed::new(client, HenryCodec::with_max_frame_size(100));

    let device_id = DeviceId::new(15).unwrap();

    // Create a message that will exceed the 100 byte limit
    let large_field = "A".repeat(200);
    let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
        .field(FieldData::new(large_field).unwrap())
        .build()
        .unwrap();

    // Should fail with FrameTooLarge error
    let result = framed.send(msg).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_codec_preserves_field_order() {
    let (mut client, mut server) = create_framed_duplex(1024);

    let device_id = DeviceId::new(15).unwrap();
    let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("field1".to_string()).unwrap())
        .field(FieldData::new("field2".to_string()).unwrap())
        .field(FieldData::new("field3".to_string()).unwrap())
        .field(FieldData::new("field4".to_string()).unwrap())
        .build()
        .unwrap();

    client.send(msg.clone()).await.unwrap();

    let received = server.next().await.unwrap().unwrap();
    assert_eq!(received.fields.len(), 4);
    assert_eq!(received.fields[0].as_str(), "field1");
    assert_eq!(received.fields[1].as_str(), "field2");
    assert_eq!(received.fields[2].as_str(), "field3");
    assert_eq!(received.fields[3].as_str(), "field4");
}

#[tokio::test]
async fn test_codec_handles_rapid_messages() {
    let (mut client, mut server) = create_framed_duplex(8192);

    let device_id = DeviceId::new(15).unwrap();

    // Send 100 messages as fast as possible
    let send_task = tokio::spawn(async move {
        for i in 0..100 {
            let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
                .field(FieldData::new(format!("{}", i)).unwrap())
                .build()
                .unwrap();

            client.send(msg).await.unwrap();
        }
    });

    // Receive all messages
    let mut count = 0;
    while let Some(result) = server.next().await {
        let msg = result.unwrap();
        assert_eq!(msg.device_id, device_id);
        assert_eq!(msg.fields[0].as_str(), format!("{}", count));
        count += 1;

        if count >= 100 {
            break;
        }
    }

    send_task.await.unwrap();
    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_codec_turnstile_access_flow() {
    let (mut turnstile, mut server) = create_framed_duplex(2048);

    let turnstile_id = DeviceId::new(15).unwrap();
    let server_id = DeviceId::new(1).unwrap();

    // 1. Turnstile requests access (card read)
    let access_request = MessageBuilder::new(turnstile_id, CommandCode::AccessRequest)
        .field(FieldData::new("11912322".to_string()).unwrap())
        .field(FieldData::new("10/05/2025 12:46:06".to_string()).unwrap())
        .field(FieldData::new("1".to_string()).unwrap())
        .field(FieldData::new("0".to_string()).unwrap())
        .build()
        .unwrap();

    turnstile.send(access_request).await.unwrap();

    // 2. Server receives request
    let received_request = server.next().await.unwrap().unwrap();
    assert_eq!(received_request.command, CommandCode::AccessRequest);
    assert_eq!(received_request.fields[0].as_str(), "11912322");

    // 3. Server grants access
    let grant_response = MessageBuilder::new(server_id, CommandCode::GrantExit)
        .field(FieldData::new("5".to_string()).unwrap())
        .field(FieldData::new("Access granted".to_string()).unwrap())
        .build()
        .unwrap();

    server.send(grant_response).await.unwrap();

    // 4. Turnstile receives grant
    let received_grant = turnstile.next().await.unwrap().unwrap();
    assert_eq!(received_grant.command, CommandCode::GrantExit);

    // 5. Turnstile waits for rotation
    let waiting = MessageBuilder::new(turnstile_id, CommandCode::WaitingRotation)
        .field(FieldData::new("".to_string()).unwrap())
        .field(FieldData::new("10/05/2025 12:46:06".to_string()).unwrap())
        .build()
        .unwrap();

    turnstile.send(waiting).await.unwrap();

    // 6. Server receives waiting status
    let received_waiting = server.next().await.unwrap().unwrap();
    assert_eq!(received_waiting.command, CommandCode::WaitingRotation);

    // 7. Turnstile confirms rotation
    let completed = MessageBuilder::new(turnstile_id, CommandCode::RotationCompleted)
        .field(FieldData::new("".to_string()).unwrap())
        .field(FieldData::new("10/05/2025 12:46:08".to_string()).unwrap())
        .build()
        .unwrap();

    turnstile.send(completed).await.unwrap();

    // 8. Server receives completion
    let received_completed = server.next().await.unwrap().unwrap();
    assert_eq!(received_completed.command, CommandCode::RotationCompleted);
}
