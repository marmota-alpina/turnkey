//! Integration tests for TcpServer
//!
//! These tests verify multi-client scenarios and the full request-response cycle.

use std::time::Duration;
use tokio::time::timeout;
use turnkey_core::DeviceId;
use turnkey_network::{TcpClient, TcpClientConfig, TcpServer, TcpServerConfig};
use turnkey_protocol::{CommandCode, MessageBuilder};

#[tokio::test]
async fn test_single_client_connection() {
    // Start server on fixed port for testing
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13001".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    // Spawn client task
    let client_task = tokio::spawn(async move {
        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        let client_config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(1000),
        };

        let mut client = TcpClient::new(client_config);
        client.connect().await.unwrap();

        // Send access request
        let device_id = DeviceId::new(15).unwrap();
        let message = MessageBuilder::new(device_id, CommandCode::AccessRequest)
            .build()
            .unwrap();

        client.send(message).await.unwrap();

        // Wait for response
        let response = client.recv().await.unwrap();
        assert_eq!(response.device_id, device_id);

        client.close().await.unwrap();
    });

    // Accept message from client
    let (device_id, message) = timeout(Duration::from_secs(5), server.accept())
        .await
        .expect("Server accept timeout")
        .unwrap();

    assert_eq!(device_id, DeviceId::new(15).unwrap());
    assert!(matches!(message.command, CommandCode::AccessRequest));

    // Send response
    let response = MessageBuilder::new(device_id, CommandCode::GrantExit)
        .build()
        .unwrap();

    server.send(device_id, response).await.unwrap();

    // Verify device is connected
    assert!(server.is_connected(device_id));

    // Wait for client to finish
    client_task.await.unwrap();
}

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    // Start server
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13002".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    // Create 3 clients with different device IDs
    let device_ids = vec![
        DeviceId::new(1).unwrap(),
        DeviceId::new(2).unwrap(),
        DeviceId::new(15).unwrap(),
    ];

    // Spawn client tasks
    let mut client_tasks = Vec::new();
    for device_id in device_ids.clone() {
        let task = tokio::spawn(async move {
            // Give server time to start
            tokio::time::sleep(Duration::from_millis(100)).await;

            let client_config = TcpClientConfig {
                server_addr,
                timeout: Duration::from_millis(1000),
            };

            let mut client = TcpClient::new(client_config);
            client.connect().await.unwrap();

            // Send access request
            let message = MessageBuilder::new(device_id, CommandCode::AccessRequest)
                .build()
                .unwrap();

            client.send(message).await.unwrap();

            // Wait for response
            let response = client.recv().await.unwrap();
            assert_eq!(response.device_id, device_id);

            client.close().await.unwrap();

            device_id
        });

        client_tasks.push(task);
    }

    // Accept messages from all clients and send responses
    let mut received_devices = Vec::new();

    for _ in 0..3 {
        let (device_id, message) = timeout(Duration::from_secs(5), server.accept())
            .await
            .expect("Server accept timeout")
            .unwrap();

        received_devices.push(device_id);
        assert!(matches!(message.command, CommandCode::AccessRequest));

        // Send response
        let response = MessageBuilder::new(device_id, CommandCode::GrantExit)
            .build()
            .unwrap();

        server.send(device_id, response).await.unwrap();
    }

    // Verify all devices connected (order doesn't matter)
    assert_eq!(received_devices.len(), device_ids.len());
    for device_id in &device_ids {
        assert!(received_devices.contains(device_id));
    }

    // Verify connected devices list
    let connected = server.connected_devices();
    assert_eq!(connected.len(), 3);

    // Wait for all clients to finish
    for task in client_tasks {
        task.await.unwrap();
    }
}

#[tokio::test]
async fn test_device_tracking() {
    // Start server
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13004".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    let device_id = DeviceId::new(5).unwrap();

    // Spawn client
    let client_task = tokio::spawn(async move {
        let client_config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(1000),
        };

        let mut client = TcpClient::new(client_config);
        client.connect().await.unwrap();

        // Send message
        let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();

        client.send(message).await.unwrap();

        // Keep connection open
        tokio::time::sleep(Duration::from_millis(100)).await;

        client.close().await.unwrap();
    });

    // Accept connection
    let (received_id, _) = timeout(Duration::from_secs(5), server.accept())
        .await
        .expect("Server accept timeout")
        .unwrap();

    assert_eq!(received_id, device_id);

    // Verify device is connected
    assert!(server.is_connected(device_id));

    // Verify device appears in connected list
    let connected = server.connected_devices();
    assert_eq!(connected.len(), 1);
    assert!(connected.contains(&device_id));

    // Wait for client to close
    client_task.await.unwrap();

    // Give server time to process disconnect
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_disconnect_device() {
    // Start server
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13005".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    let device_id = DeviceId::new(7).unwrap();

    // Spawn client
    let client_task = tokio::spawn(async move {
        let client_config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(1000),
        };

        let mut client = TcpClient::new(client_config);
        client.connect().await.unwrap();

        // Send message
        let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();

        client.send(message).await.unwrap();

        // Wait to be disconnected
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Accept connection
    timeout(Duration::from_secs(5), server.accept())
        .await
        .expect("Server accept timeout")
        .unwrap();

    // Verify connected
    assert!(server.is_connected(device_id));

    // Disconnect device
    server.disconnect(device_id).await.unwrap();

    // Verify disconnected
    assert!(!server.is_connected(device_id));

    // Verify not in connected list
    let connected = server.connected_devices();
    assert_eq!(connected.len(), 0);

    // Wait for client
    let _ = client_task.await;
}

#[tokio::test]
async fn test_send_to_disconnected_device() {
    // Start server
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13006".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config).await.unwrap();

    let device_id = DeviceId::new(99).unwrap();
    let message = MessageBuilder::new(device_id, CommandCode::GrantEntry)
        .build()
        .unwrap();

    // Try to send to non-existent device
    let result = server.send(device_id, message).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_disconnect_already_disconnected() {
    // Start server
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13007".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config).await.unwrap();

    let device_id = DeviceId::new(88).unwrap();

    // Try to disconnect non-existent device
    let result = server.disconnect(device_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_full_request_response_cycle() {
    // Start server
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13008".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    let device_id = DeviceId::new(42).unwrap();

    // Spawn client task
    let client_task = tokio::spawn(async move {
        let client_config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(2000),
        };

        let mut client = TcpClient::new(client_config);
        client.connect().await.unwrap();

        // Send 3 requests
        for i in 0..3 {
            let message = MessageBuilder::new(device_id, CommandCode::AccessRequest)
                .build()
                .unwrap();

            client.send(message).await.unwrap();

            // Wait for response
            let response = client.recv().await.unwrap();
            assert_eq!(response.device_id, device_id);

            println!("Client: Received response {}", i + 1);
        }

        client.close().await.unwrap();
    });

    // Process 3 request-response cycles
    // First message comes from accept() (new connection)
    let (received_id, message) = timeout(Duration::from_secs(5), server.accept())
        .await
        .expect("Server accept timeout")
        .unwrap();

    assert_eq!(received_id, device_id);
    assert!(matches!(message.command, CommandCode::AccessRequest));
    println!("Server: Received request 1");

    let response = MessageBuilder::new(device_id, CommandCode::GrantEntry)
        .build()
        .unwrap();
    server.send(device_id, response).await.unwrap();

    // Subsequent messages use recv() (existing connection)
    for i in 1..3 {
        let message = timeout(Duration::from_secs(5), server.recv(device_id))
            .await
            .expect("Server recv timeout")
            .unwrap()
            .expect("Connection closed");

        assert_eq!(message.device_id, device_id);
        assert!(matches!(message.command, CommandCode::AccessRequest));
        println!("Server: Received request {}", i + 1);

        let response = MessageBuilder::new(device_id, CommandCode::GrantEntry)
            .build()
            .unwrap();
        server.send(device_id, response).await.unwrap();
    }

    // Wait for client to finish
    client_task.await.unwrap();
}

#[tokio::test]
async fn test_duplicate_device_id_rejected() {
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13010".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;
    let device_id = DeviceId::new(15).unwrap();

    // First client connects with device ID 15
    let _client1_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(1000),
        };
        let mut client = TcpClient::new(config);
        client.connect().await.unwrap();

        let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();
        client.send(message).await.unwrap();

        // Keep connection alive
        tokio::time::sleep(Duration::from_secs(1)).await;
    });

    // Accept first connection
    let (received_id, _) = timeout(Duration::from_secs(5), server.accept())
        .await
        .expect("Server accept timeout")
        .unwrap();
    assert_eq!(received_id, device_id);
    assert!(server.is_connected(device_id));

    // Second client attempts same device ID (server should reject and continue)
    let _client2_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(500),
        };
        let mut client = TcpClient::new(config);
        client.connect().await.unwrap();

        let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();
        client.send(message).await.unwrap_or(());
    });

    // Give time for duplicate connection attempt
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Verify first connection still active and only one connection exists
    assert!(server.is_connected(device_id));
    assert_eq!(server.connected_devices().len(), 1);
}

#[tokio::test]
async fn test_max_connections_enforced() {
    let max_conns = 3;
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13011".parse().unwrap(),
        max_connections: max_conns,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    // Spawn max_connections clients
    let mut client_tasks = Vec::new();
    for i in 1..=max_conns {
        let device_id = DeviceId::new(i as u8).unwrap();
        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let config = TcpClientConfig {
                server_addr,
                timeout: Duration::from_millis(1000),
            };
            let mut client = TcpClient::new(config);
            client.connect().await.unwrap();

            let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
                .build()
                .unwrap();
            client.send(message).await.unwrap();

            // Keep connection alive
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        client_tasks.push(task);
    }

    // Accept all max_connections clients
    for _ in 0..max_conns {
        timeout(Duration::from_secs(5), server.accept())
            .await
            .expect("Server accept timeout")
            .unwrap();
    }

    assert_eq!(server.connected_devices().len(), max_conns);

    // Try to connect one more client (should be rejected silently)
    let device_id_overflow = DeviceId::new(99).unwrap();
    let _overflow_client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(500),
        };
        let mut client = TcpClient::new(config);
        client.connect().await.unwrap();

        let message = MessageBuilder::new(device_id_overflow, CommandCode::QueryStatus)
            .build()
            .unwrap();
        client.send(message).await.unwrap_or(());
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    // Verify max connections still maintained (overflow was rejected)
    assert_eq!(server.connected_devices().len(), max_conns);

    for task in client_tasks {
        task.abort();
    }
}

#[tokio::test]
async fn test_server_continues_after_rejection() {
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13012".parse().unwrap(),
        max_connections: 2,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    // Connect two clients (fill capacity)
    let device1 = DeviceId::new(1).unwrap();
    let device2 = DeviceId::new(2).unwrap();

    for device_id in [device1, device2] {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let config = TcpClientConfig {
                server_addr,
                timeout: Duration::from_millis(1000),
            };
            let mut client = TcpClient::new(config);
            client.connect().await.unwrap();
            let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
                .build()
                .unwrap();
            client.send(msg).await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
    }

    // Accept both
    server.accept().await.unwrap();
    server.accept().await.unwrap();

    // Try third client (rejected due to max connections)
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(500),
        };
        let mut client = TcpClient::new(config);
        client.connect().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Disconnect one device
    server.disconnect(device1).await.unwrap();

    // Now connect a new client (should succeed)
    let device3 = DeviceId::new(3).unwrap();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(400)).await;
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(1000),
        };
        let mut client = TcpClient::new(config);
        client.connect().await.unwrap();
        let msg = MessageBuilder::new(device3, CommandCode::QueryStatus)
            .build()
            .unwrap();
        client.send(msg).await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Should accept new connection successfully
    let (new_id, _) = timeout(Duration::from_secs(2), server.accept())
        .await
        .expect("Failed to accept after rejection")
        .unwrap();

    assert_eq!(new_id, device3);
    assert_eq!(server.connected_devices().len(), 2);
}

#[tokio::test]
async fn test_connection_info_metadata() {
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13013".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;
    let device_id = DeviceId::new(42).unwrap();

    // Spawn client
    let _client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(1000),
        };
        let mut client = TcpClient::new(config);
        client.connect().await.unwrap();

        let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .build()
            .unwrap();
        client.send(message).await.unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;
    });

    // Accept connection
    let (received_id, _) = timeout(Duration::from_secs(5), server.accept())
        .await
        .expect("Server accept timeout")
        .unwrap();

    assert_eq!(received_id, device_id);

    // Test connection_info
    let info = server
        .connection_info(device_id)
        .expect("Connection should exist");
    assert_eq!(info.device_id, device_id);
    assert!(info.uptime.num_milliseconds() >= 0);

    // Test all_connections_info
    let all_info = server.all_connections_info();
    assert_eq!(all_info.len(), 1);
    assert_eq!(all_info[0].device_id, device_id);

    // Test info for non-existent device
    let non_existent = DeviceId::new(99).unwrap();
    assert!(server.connection_info(non_existent).is_none());
}

#[tokio::test]
async fn test_recv_any_single_device() {
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13014".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;
    let device_id = DeviceId::new(7).unwrap();

    // Spawn client that sends multiple messages
    let client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let config = TcpClientConfig {
            server_addr,
            timeout: Duration::from_millis(1000),
        };
        let mut client = TcpClient::new(config);
        client.connect().await.unwrap();

        // Send 3 messages
        for _ in 0..3 {
            let message = MessageBuilder::new(device_id, CommandCode::AccessRequest)
                .build()
                .unwrap();
            client.send(message).await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        client.close().await.unwrap();
    });

    // Use recv_any to receive all 3 messages
    for i in 0..3 {
        let (recv_id, message) = timeout(Duration::from_secs(2), server.recv_any())
            .await
            .expect("recv_any timeout")
            .unwrap();

        assert_eq!(recv_id, device_id);
        assert!(matches!(message.command, CommandCode::AccessRequest));
        println!("Received message {} via recv_any", i + 1);
    }

    client_task.await.unwrap();
}

#[tokio::test]
async fn test_recv_any_multiple_devices() {
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13015".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    let devices = vec![
        DeviceId::new(1).unwrap(),
        DeviceId::new(2).unwrap(),
        DeviceId::new(3).unwrap(),
    ];

    // Spawn multiple clients
    let mut client_tasks = Vec::new();
    for device_id in devices.clone() {
        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let config = TcpClientConfig {
                server_addr,
                timeout: Duration::from_millis(1000),
            };
            let mut client = TcpClient::new(config);
            client.connect().await.unwrap();

            // Each client sends one message
            let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
                .build()
                .unwrap();
            client.send(message).await.unwrap();

            tokio::time::sleep(Duration::from_millis(500)).await;
            client.close().await.unwrap();
        });
        client_tasks.push(task);
    }

    // Use recv_any to receive from all devices
    let mut received_ids = Vec::new();
    for _ in 0..3 {
        let (device_id, _) = timeout(Duration::from_secs(5), server.recv_any())
            .await
            .expect("recv_any timeout")
            .unwrap();
        received_ids.push(device_id);
    }

    // Verify we received from all devices (order doesn't matter)
    assert_eq!(received_ids.len(), 3);
    for device_id in &devices {
        assert!(received_ids.contains(device_id));
    }

    for task in client_tasks {
        task.await.unwrap();
    }
}

#[tokio::test]
async fn test_rapid_connect_disconnect() {
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13016".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    // Spawn 5 clients that connect, send message, and disconnect rapidly
    let mut client_tasks = Vec::new();
    for i in 1..=5 {
        let device_id = DeviceId::new(i).unwrap();
        let task = tokio::spawn(async move {
            // Stagger connections slightly
            tokio::time::sleep(Duration::from_millis(i as u64 * 50)).await;

            let config = TcpClientConfig {
                server_addr,
                timeout: Duration::from_millis(1000),
            };
            let mut client = TcpClient::new(config);
            client.connect().await.unwrap();

            let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
                .build()
                .unwrap();
            client.send(message).await.unwrap();

            // Disconnect immediately
            client.close().await.unwrap();
        });
        client_tasks.push(task);
    }

    // Accept all 5 connections
    let mut received_count = 0;
    for _ in 0..5 {
        match timeout(Duration::from_secs(3), server.recv_any()).await {
            Ok(Ok(_)) => {
                received_count += 1;
            }
            Ok(Err(e)) => {
                println!("Error receiving: {}", e);
            }
            Err(_) => {
                println!("Timeout waiting for message");
                break;
            }
        }
    }

    assert_eq!(received_count, 5, "Should receive 5 messages");

    for task in client_tasks {
        task.await.unwrap();
    }

    // Give time for disconnections to be processed
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Most or all connections should be closed
    // Note: Due to timing, some connections might still be closing
    assert!(
        server.connected_devices().len() <= 1,
        "Expected 0-1 connections, got {}",
        server.connected_devices().len()
    );
}

#[tokio::test]
async fn test_connection_info_tracking() {
    let server_config = TcpServerConfig {
        bind_addr: "127.0.0.1:13017".parse().unwrap(),
        max_connections: 10,
    };

    let mut server = TcpServer::bind(server_config.clone()).await.unwrap();
    let server_addr = server_config.bind_addr;

    let device1 = DeviceId::new(10).unwrap();
    let device2 = DeviceId::new(20).unwrap();

    // Connect two devices
    for device_id in [device1, device2] {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let config = TcpClientConfig {
                server_addr,
                timeout: Duration::from_millis(1000),
            };
            let mut client = TcpClient::new(config);
            client.connect().await.unwrap();

            let message = MessageBuilder::new(device_id, CommandCode::QueryStatus)
                .build()
                .unwrap();
            client.send(message).await.unwrap();

            tokio::time::sleep(Duration::from_secs(2)).await;
        });
    }

    // Accept both connections
    server.recv_any().await.unwrap();
    server.recv_any().await.unwrap();

    // Check connection info
    let all_info = server.all_connections_info();
    assert_eq!(all_info.len(), 2);

    // Verify both devices are tracked
    let device1_info = server
        .connection_info(device1)
        .expect("Device 1 should be connected");
    let device2_info = server
        .connection_info(device2)
        .expect("Device 2 should be connected");

    assert_eq!(device1_info.device_id, device1);
    assert_eq!(device2_info.device_id, device2);

    // Verify uptime is reasonable (should be >= 0 and < 2 seconds at this point)
    assert!(device1_info.uptime.num_milliseconds() >= 0);
    assert!(device1_info.uptime.num_seconds() < 2);
}
