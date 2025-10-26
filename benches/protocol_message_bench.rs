//! Benchmark tests for protocol message creation and parsing.
//!
//! These benchmarks establish performance baselines for critical protocol
//! operations to detect performance regressions and validate that latency
//! requirements (<10ms) are met.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use turnkey_core::{AccessDirection, DeviceId, HenryTimestamp, ReaderType};
use turnkey_protocol::{
    CommandCode, FieldData, MessageBuilder,
    commands::access::{AccessDecision, AccessRequest, AccessResponse},
};

/// Benchmark: Create access request message.
///
/// Tests the performance of building a complete access request message
/// using MessageBuilder. This is on the critical path for every card read.
fn bench_create_access_request(c: &mut Criterion) {
    let device_id = DeviceId::new(15).expect("Valid device ID");
    let card_number = "12345678";

    c.bench_function("create_access_request", |b| {
        b.iter(|| {
            let timestamp = HenryTimestamp::now();
            MessageBuilder::new(device_id, CommandCode::AccessRequest)
                .field(FieldData::new(card_number.to_string()).expect("Valid card number"))
                .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
                .field(
                    FieldData::new(AccessDirection::Entry.to_u8().to_string())
                        .expect("Valid direction"),
                )
                .field(
                    FieldData::new(ReaderType::Rfid.to_u8().to_string())
                        .expect("Valid reader type"),
                )
                .build()
                .expect("Valid message")
        })
    });
}

/// Benchmark: Parse access request from protocol fields.
///
/// Tests the performance of parsing a received access request message.
/// This is critical for server-side validation latency.
fn bench_parse_access_request(c: &mut Criterion) {
    // Pre-create the message
    let device_id = DeviceId::new(15).expect("Valid device ID");
    let timestamp = HenryTimestamp::now();
    let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("12345678".to_string()).expect("Valid card"))
        .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
        .field(FieldData::new("1".to_string()).expect("Valid direction"))
        .field(FieldData::new("0".to_string()).expect("Valid reader type"))
        .build()
        .expect("Valid message");

    let fields: Vec<String> = msg.fields.iter().map(|f| f.as_str().to_string()).collect();

    c.bench_function("parse_access_request", |b| {
        b.iter(|| AccessRequest::parse(black_box(&fields)).expect("Valid request"))
    });
}

/// Benchmark: Create access response message.
///
/// Tests the performance of building grant/deny response messages.
fn bench_create_access_response(c: &mut Criterion) {
    let device_id = DeviceId::new(15).expect("Valid device ID");

    c.bench_function("create_access_response", |b| {
        b.iter(|| {
            MessageBuilder::new(device_id, CommandCode::GrantEntry)
                .field(FieldData::new("5".to_string()).expect("Valid timeout"))
                .field(FieldData::new("Acesso liberado".to_string()).expect("Valid message"))
                .build()
                .expect("Valid message")
        })
    });
}

/// Benchmark: Serialize message to wire format.
///
/// Tests the performance of converting a Message to the string format
/// sent over TCP/IP.
fn bench_message_serialization(c: &mut Criterion) {
    let device_id = DeviceId::new(15).expect("Valid device ID");
    let timestamp = HenryTimestamp::now();
    let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("12345678".to_string()).expect("Valid card"))
        .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
        .field(FieldData::new("1".to_string()).expect("Valid direction"))
        .field(FieldData::new("0".to_string()).expect("Valid reader type"))
        .build()
        .expect("Valid message");

    c.bench_function("serialize_message", |b| {
        b.iter(|| black_box(msg.to_string()))
    });
}

/// Benchmark: Message display (to_string conversion).
///
/// Tests the performance of converting Message to String for transmission.
/// Note: Message parsing from wire format is done by StreamParser in the protocol layer.
fn bench_message_display(c: &mut Criterion) {
    let device_id = DeviceId::new(15).expect("Valid device ID");
    let timestamp = HenryTimestamp::now();
    let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("12345678".to_string()).expect("Valid card"))
        .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
        .field(FieldData::new("1".to_string()).expect("Valid direction"))
        .field(FieldData::new("0".to_string()).expect("Valid reader type"))
        .build()
        .expect("Valid message");

    c.bench_function("message_display", |b| {
        b.iter(|| black_box(format!("{}", msg)))
    });
}

/// Benchmark: Complete access request creation and serialization.
///
/// Tests the full cycle: create → serialize → parse fields.
/// This measures end-to-end latency for a single access transaction.
fn bench_access_request_cycle(c: &mut Criterion) {
    let device_id = DeviceId::new(15).expect("Valid device ID");
    let card_number = "12345678";

    c.bench_function("access_request_cycle", |b| {
        b.iter(|| {
            // 1. Create message
            let timestamp = HenryTimestamp::now();
            let msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
                .field(FieldData::new(card_number.to_string()).expect("Valid card"))
                .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
                .field(
                    FieldData::new(AccessDirection::Entry.to_u8().to_string())
                        .expect("Valid direction"),
                )
                .field(
                    FieldData::new(ReaderType::Rfid.to_u8().to_string())
                        .expect("Valid reader type"),
                )
                .build()
                .expect("Valid message");

            // 2. Serialize to string
            let _wire = msg.to_string();

            // 3. Parse fields from message
            let fields: Vec<String> = msg.fields.iter().map(|f| f.as_str().to_string()).collect();
            let _request = AccessRequest::parse(&fields).expect("Valid request");
        })
    });
}

/// Benchmark: Throughput test - multiple messages.
///
/// Tests how many messages per second can be processed.
/// Target: 1000+ messages/second (per CLAUDE.md requirements).
fn bench_message_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let device_id = DeviceId::new(15).expect("Valid device ID");
            b.iter(|| {
                for i in 0..count {
                    let card = format!("{:08}", i);
                    let timestamp = HenryTimestamp::now();
                    let _msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
                        .field(FieldData::new(card).expect("Valid card"))
                        .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
                        .field(FieldData::new("1".to_string()).expect("Valid direction"))
                        .field(FieldData::new("0".to_string()).expect("Valid reader type"))
                        .build()
                        .expect("Valid message");
                }
            })
        });
    }
    group.finish();
}

/// Benchmark: Message creation with varying card number lengths.
///
/// Tests if performance varies with card number length (3-20 chars).
fn bench_varying_card_lengths(c: &mut Criterion) {
    let mut group = c.benchmark_group("card_length");
    let device_id = DeviceId::new(15).expect("Valid device ID");

    for length in [3, 8, 12, 20].iter() {
        let card = "1".repeat(*length);
        group.bench_with_input(BenchmarkId::from_parameter(length), length, |b, _| {
            b.iter(|| {
                let timestamp = HenryTimestamp::now();
                MessageBuilder::new(device_id, CommandCode::AccessRequest)
                    .field(FieldData::new(card.clone()).expect("Valid card"))
                    .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
                    .field(FieldData::new("1".to_string()).expect("Valid direction"))
                    .field(FieldData::new("0".to_string()).expect("Valid reader type"))
                    .build()
                    .expect("Valid message")
            })
        });
    }
    group.finish();
}

/// Benchmark: AccessResponse helper methods.
///
/// Compares performance of different AccessResponse creation methods.
fn bench_access_response_helpers(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_helpers");

    // Test grant_entry helper
    group.bench_function("grant_entry_helper", |b| {
        b.iter(|| AccessResponse::grant_entry(black_box("Acesso liberado".to_string())))
    });

    // Test grant_exit helper
    group.bench_function("grant_exit_helper", |b| {
        b.iter(|| AccessResponse::grant_exit(black_box("Acesso liberado".to_string())))
    });

    // Test deny helper
    group.bench_function("deny_helper", |b| {
        b.iter(|| AccessResponse::deny(black_box("Acesso negado".to_string())))
    });

    // Test explicit constructor
    group.bench_function("explicit_constructor", |b| {
        b.iter(|| {
            AccessResponse::new(
                black_box(AccessDecision::GrantEntry),
                black_box(5),
                black_box("Acesso liberado".to_string()),
            )
        })
    });

    group.finish();
}

/// Benchmark: Complete online validation flow.
///
/// Simulates the complete round-trip latency for an online validation:
/// 1. Turnstile creates access request
/// 2. Server parses request
/// 3. Server creates grant/deny response
/// 4. Turnstile parses response
///
/// This measures the critical path latency excluding network I/O.
/// Target: <10ms per CLAUDE.md requirements (including network, this should be <5ms).
fn bench_online_validation_flow(c: &mut Criterion) {
    let device_id = DeviceId::new(15).expect("Valid device ID");
    let card_number = "12345678";

    c.bench_function("online_validation_flow", |b| {
        b.iter(|| {
            // 1. Turnstile: Create access request
            let timestamp = HenryTimestamp::now();
            let request_msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
                .field(FieldData::new(card_number.to_string()).expect("Valid card"))
                .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
                .field(FieldData::new("1".to_string()).expect("Valid direction"))
                .field(FieldData::new("1".to_string()).expect("Valid reader type"))
                .build()
                .expect("Valid message");

            // 2. Server: Parse request
            let fields: Vec<String> = request_msg
                .fields
                .iter()
                .map(|f| f.as_str().to_string())
                .collect();
            let _request = AccessRequest::parse(&fields).expect("Valid request");

            // 3. Server: Create response (grant scenario)
            let response_msg = MessageBuilder::new(device_id, CommandCode::GrantEntry)
                .field(FieldData::new("5".to_string()).expect("Valid timeout"))
                .field(FieldData::new("Acesso liberado".to_string()).expect("Valid message"))
                .build()
                .expect("Valid message");

            // 4. Turnstile: Parse response
            let _response_fields: Vec<String> = response_msg
                .fields
                .iter()
                .map(|f| f.as_str().to_string())
                .collect();

            // Simulate decision extraction
            let decision = match response_msg.command {
                CommandCode::GrantEntry => AccessDecision::GrantEntry,
                CommandCode::GrantExit => AccessDecision::GrantExit,
                CommandCode::GrantBoth => AccessDecision::GrantBoth,
                CommandCode::DenyAccess => AccessDecision::Deny,
                _ => panic!("Invalid command"),
            };

            black_box(decision);
        })
    });
}

/// Benchmark: Offline validation simulation.
///
/// Tests the latency of local card validation without network round-trip.
/// This is the fallback path when online validation times out.
fn bench_offline_validation_simulation(c: &mut Criterion) {
    use std::collections::HashMap;

    // Pre-populate a mock "local database"
    let mut local_cards = HashMap::new();
    for i in 0..1000 {
        local_cards.insert(format!("{:08}", i), true);
    }

    c.bench_function("offline_validation_lookup", |b| {
        b.iter(|| {
            let card = "00000042";
            let is_valid = local_cards.get(card).copied().unwrap_or(false);
            black_box(is_valid);
        })
    });
}

/// Benchmark: Complete online validation with denial.
///
/// Tests the denial path performance to ensure it's not slower than grant.
fn bench_online_validation_denial(c: &mut Criterion) {
    let device_id = DeviceId::new(15).expect("Valid device ID");
    let card_number = "99999999"; // Invalid card

    c.bench_function("online_validation_denial", |b| {
        b.iter(|| {
            // 1. Turnstile: Create access request
            let timestamp = HenryTimestamp::now();
            let request_msg = MessageBuilder::new(device_id, CommandCode::AccessRequest)
                .field(FieldData::new(card_number.to_string()).expect("Valid card"))
                .field(FieldData::new(timestamp.format()).expect("Valid timestamp"))
                .field(FieldData::new("1".to_string()).expect("Valid direction"))
                .field(FieldData::new("1".to_string()).expect("Valid reader type"))
                .build()
                .expect("Valid message");

            // 2. Server: Parse request
            let fields: Vec<String> = request_msg
                .fields
                .iter()
                .map(|f| f.as_str().to_string())
                .collect();
            let _request = AccessRequest::parse(&fields).expect("Valid request");

            // 3. Server: Create denial response
            let _ = MessageBuilder::new(device_id, CommandCode::DenyAccess)
                .field(FieldData::new("0".to_string()).expect("Valid timeout"))
                .field(FieldData::new("Acesso negado".to_string()).expect("Valid message"))
                .build()
                .expect("Valid message");

            // 4. Turnstile: Parse response
            let decision = AccessDecision::Deny;

            black_box(decision);
        })
    });
}

criterion_group!(
    benches,
    bench_create_access_request,
    bench_parse_access_request,
    bench_create_access_response,
    bench_message_serialization,
    bench_message_display,
    bench_access_request_cycle,
    bench_message_throughput,
    bench_varying_card_lengths,
    bench_access_response_helpers,
    bench_online_validation_flow,
    bench_offline_validation_simulation,
    bench_online_validation_denial,
);
criterion_main!(benches);
