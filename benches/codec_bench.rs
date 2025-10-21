//! Performance benchmarks for HenryCodec.
//!
//! These benchmarks measure the throughput and latency of the codec
//! to ensure it meets the performance target of 1000+ messages/second.
//!
//! Run benchmarks with:
//! ```sh
//! cargo bench --bench codec_bench
//! ```

use bytes::BytesMut;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tokio_util::codec::{Decoder, Encoder};
use turnkey_core::DeviceId;
use turnkey_protocol::{CommandCode, FieldData, HenryCodec, Message, MessageBuilder};

/// Create a simple query status message for benchmarking.
fn create_simple_message() -> Message {
    let device_id = DeviceId::new(15).unwrap();
    MessageBuilder::new(device_id, CommandCode::QueryStatus)
        .build()
        .unwrap()
}

/// Create an access request message with multiple fields.
fn create_complex_message() -> Message {
    let device_id = DeviceId::new(15).unwrap();
    MessageBuilder::new(device_id, CommandCode::AccessRequest)
        .field(FieldData::new("12345678".to_string()).unwrap())
        .field(FieldData::new("10/05/2025 12:46:06".to_string()).unwrap())
        .field(FieldData::new("1".to_string()).unwrap())
        .field(FieldData::new("0".to_string()).unwrap())
        .build()
        .unwrap()
}

/// Benchmark encoding a simple message.
fn bench_encode_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_simple");
    group.throughput(Throughput::Elements(1));

    let msg = create_simple_message();

    group.bench_function("encode_simple_message", |b| {
        b.iter(|| {
            let mut codec = HenryCodec::new();
            let mut buffer = BytesMut::new();
            codec.encode(black_box(msg.clone()), &mut buffer).unwrap();
            black_box(buffer);
        });
    });

    group.finish();
}

/// Benchmark encoding a complex message with multiple fields.
fn bench_encode_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_complex");
    group.throughput(Throughput::Elements(1));

    let msg = create_complex_message();

    group.bench_function("encode_complex_message", |b| {
        b.iter(|| {
            let mut codec = HenryCodec::new();
            let mut buffer = BytesMut::new();
            codec.encode(black_box(msg.clone()), &mut buffer).unwrap();
            black_box(buffer);
        });
    });

    group.finish();
}

/// Benchmark decoding a simple message.
fn bench_decode_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_simple");
    group.throughput(Throughput::Elements(1));

    // Pre-encode the message
    let msg = create_simple_message();
    let mut codec = HenryCodec::new();
    let mut encoded = BytesMut::new();
    codec.encode(msg, &mut encoded).unwrap();
    let encoded_bytes = encoded.freeze();

    group.bench_function("decode_simple_message", |b| {
        b.iter(|| {
            let mut codec = HenryCodec::new();
            let mut buffer = BytesMut::from(&encoded_bytes[..]);
            let result = codec.decode(&mut buffer).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark decoding a complex message.
fn bench_decode_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_complex");
    group.throughput(Throughput::Elements(1));

    // Pre-encode the message
    let msg = create_complex_message();
    let mut codec = HenryCodec::new();
    let mut encoded = BytesMut::new();
    codec.encode(msg, &mut encoded).unwrap();
    let encoded_bytes = encoded.freeze();

    group.bench_function("decode_complex_message", |b| {
        b.iter(|| {
            let mut codec = HenryCodec::new();
            let mut buffer = BytesMut::from(&encoded_bytes[..]);
            let result = codec.decode(&mut buffer).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark roundtrip encoding and decoding.
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");
    group.throughput(Throughput::Elements(1));

    let msg = create_complex_message();

    group.bench_function("roundtrip_complex_message", |b| {
        b.iter(|| {
            let mut encoder = HenryCodec::new();
            let mut decoder = HenryCodec::new();
            let mut buffer = BytesMut::new();

            // Encode
            encoder.encode(black_box(msg.clone()), &mut buffer).unwrap();

            // Decode
            let result = decoder.decode(&mut buffer).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark encoding multiple messages in sequence.
fn bench_encode_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_batch");

    for batch_size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                let messages: Vec<Message> = (0..size).map(|_| create_simple_message()).collect();

                b.iter(|| {
                    let mut codec = HenryCodec::new();
                    let mut buffer = BytesMut::new();

                    for msg in &messages {
                        codec.encode(black_box(msg.clone()), &mut buffer).unwrap();
                    }

                    black_box(buffer);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark decoding multiple messages in sequence.
fn bench_decode_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_batch");

    for batch_size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        // Pre-encode all messages
        let mut codec = HenryCodec::new();
        let mut encoded = BytesMut::new();

        for _ in 0..*batch_size {
            codec.encode(create_simple_message(), &mut encoded).unwrap();
        }

        let encoded_bytes = encoded.freeze();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, _| {
                b.iter(|| {
                    let mut codec = HenryCodec::new();
                    let mut buffer = BytesMut::from(&encoded_bytes[..]);
                    let mut count = 0;

                    while let Ok(Some(_)) = codec.decode(&mut buffer) {
                        count += 1;
                    }

                    black_box(count);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark throughput - messages per second.
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Elements(1000));

    let messages: Vec<Message> = (0..1000).map(|_| create_simple_message()).collect();

    group.bench_function("encode_1000_messages", |b| {
        b.iter(|| {
            let mut codec = HenryCodec::new();
            let mut buffer = BytesMut::new();

            for msg in &messages {
                codec.encode(black_box(msg.clone()), &mut buffer).unwrap();
            }

            black_box(buffer);
        });
    });

    group.finish();
}

/// Benchmark latency - time per message.
fn bench_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency");
    group.throughput(Throughput::Elements(1));

    let msg = create_simple_message();

    group.bench_function("single_message_latency", |b| {
        b.iter(|| {
            let mut encoder = HenryCodec::new();
            let mut decoder = HenryCodec::new();
            let mut buffer = BytesMut::new();

            encoder.encode(black_box(msg.clone()), &mut buffer).unwrap();
            let result = decoder.decode(&mut buffer).unwrap();

            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark with different message sizes.
fn bench_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_sizes");

    for field_size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Bytes(*field_size as u64));

        let data = "A".repeat(*field_size);
        let device_id = DeviceId::new(15).unwrap();
        let msg = MessageBuilder::new(device_id, CommandCode::QueryStatus)
            .field(FieldData::new(data).unwrap())
            .build()
            .unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(field_size),
            field_size,
            |b, _| {
                b.iter(|| {
                    let mut encoder = HenryCodec::new();
                    let mut decoder = HenryCodec::new();
                    let mut buffer = BytesMut::new();

                    encoder.encode(black_box(msg.clone()), &mut buffer).unwrap();
                    let result = decoder.decode(&mut buffer).unwrap();

                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark decoding with partial frames across multiple decode calls.
///
/// This benchmark simulates realistic TCP streaming where frames arrive
/// in small chunks, requiring multiple decode() calls to assemble a
/// complete message.
fn bench_decode_partial_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_partial_streaming");
    group.throughput(Throughput::Elements(1));

    // Pre-encode a complex message
    let msg = create_complex_message();
    let mut encoder = HenryCodec::new();
    let mut buffer = BytesMut::new();
    encoder.encode(msg, &mut buffer).unwrap();
    let full_frame = buffer.freeze();

    // Test different chunk sizes to simulate various network conditions
    for chunk_size in [8, 16, 32].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("chunk_{}_bytes", chunk_size)),
            chunk_size,
            |b, &size| {
                b.iter(|| {
                    let mut codec = HenryCodec::new();
                    let mut result = None;

                    // Feed frame in small chunks, simulating TCP stream
                    for chunk in full_frame.chunks(size) {
                        let mut buf = BytesMut::from(chunk);
                        if let Ok(Some(msg)) = codec.decode(&mut buf) {
                            result = Some(msg);
                            break;
                        }
                    }

                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_simple,
    bench_encode_complex,
    bench_decode_simple,
    bench_decode_complex,
    bench_roundtrip,
    bench_encode_batch,
    bench_decode_batch,
    bench_throughput,
    bench_latency,
    bench_message_sizes,
    bench_decode_partial_streaming,
);

criterion_main!(benches);
