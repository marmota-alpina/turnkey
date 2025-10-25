//! Performance benchmarks for validation functions.
//!
//! These benchmarks measure the impact of zero-copy optimization in validation
//! functions, comparing allocation-free validation (&str returns) against
//! allocation-based approaches (String returns).
//!
//! # Key Metrics
//!
//! - **Throughput**: Operations per second for validation
//! - **Latency**: Time per validation operation
//! - **Memory**: Allocation impact on performance
//!
//! # Run Benchmarks
//!
//! ## Basic Usage
//!
//! ```sh
//! # Run all validation benchmarks
//! cargo bench --bench validation_bench
//!
//! # Run specific benchmark group
//! cargo bench --bench validation_bench -- card_number
//!
//! # Generate detailed statistical analysis
//! cargo bench --bench validation_bench -- --verbose
//! ```
//!
//! ## Baseline Comparison Workflow
//!
//! Track performance changes over time using baselines:
//!
//! ```sh
//! # Step 1: Save baseline before making changes
//! cargo bench --bench validation_bench -- --save-baseline before-optimization
//!
//! # Step 2: Make your code changes (e.g., refactoring, optimization)
//! # ... edit code ...
//!
//! # Step 3: Compare current performance against baseline
//! cargo bench --bench validation_bench -- --baseline before-optimization
//!
//! # Step 4: If improvements are good, update the baseline
//! cargo bench --bench validation_bench -- --save-baseline after-optimization
//! ```
//!
//! ## Interpreting Results
//!
//! Criterion reports performance changes with confidence intervals:
//!
//! - **No change**: `[-5.0% +5.0%]` - Performance unchanged within statistical noise
//! - **Improvement**: `[-15.0% -8.0%]` - Reliably faster (negative is better)
//! - **Regression**: `[+8.0% +15.0%]` - Reliably slower (positive is worse, investigate!)
//!
//! ### Example Output
//!
//! ```text
//! card_number_validation/zero_copy/typical
//!   time:   [42.3 ns 42.8 ns 43.2 ns]
//!   change: [-12.5% -10.2% -8.1%] (improvement)
//! ```
//!
//! This indicates the benchmark runs ~10% faster than baseline with high confidence.
//!
//! ## CI Integration
//!
//! For continuous performance monitoring:
//!
//! ```sh
//! # In CI pipeline, save baseline from main branch
//! git checkout main
//! cargo bench --bench validation_bench -- --save-baseline main
//!
//! # Switch to PR branch and compare
//! git checkout feature-branch
//! cargo bench --bench validation_bench -- --baseline main
//! ```
//!
//! # Expected Results
//!
//! Zero-copy validation should show:
//! - **2-3x faster throughput** (no allocation overhead)
//! - **~50-100ns lower latency** per operation
//! - **Zero heap allocations** (confirmed by profiling tools like valgrind/massif)

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use turnkey_protocol::validation::{validate_card_number, validate_field, validate_field_lengths};

/// Benchmark card number validation with zero-copy optimization.
///
/// Tests the performance of `validate_card_number()` which returns `Result<&str>`
/// instead of `Result<String>`, eliminating allocation overhead.
fn bench_card_number_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("card_number_validation");
    group.throughput(Throughput::Elements(1));

    // Test cases with different card number lengths
    let test_cases = vec![
        ("min_length", "123"),                  // 3 chars (minimum)
        ("typical", "12345678"),                // 8 chars (typical RFID)
        ("max_length", "12345678901234567890"), // 20 chars (maximum)
    ];

    for (name, card) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("zero_copy", name),
            &card,
            |b, &card_num| {
                b.iter(|| {
                    // Zero-copy: returns &str without allocation
                    let result = validate_card_number(black_box(card_num));
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark card number validation with different validation scenarios.
///
/// Compares performance across:
/// - Valid cards (fast path)
/// - Invalid length (early rejection)
/// - Invalid delimiter (early rejection)
fn bench_card_number_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("card_number_scenarios");
    group.throughput(Throughput::Elements(1));

    let scenarios = vec![
        ("valid_card", "12345678", true),
        ("too_short", "12", false),
        ("too_long", "123456789012345678901", false),
        ("with_delimiter", "1234]567", false),
        ("empty", "", true),
    ];

    for (name, card, _expected_valid) in scenarios {
        group.bench_function(name, |b| {
            b.iter(|| {
                let result = validate_card_number(black_box(card));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark field validation for protocol delimiter detection.
///
/// Tests the performance of `validate_field()` which checks for
/// reserved protocol delimiters (], +, [).
fn bench_field_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_validation");
    group.throughput(Throughput::Elements(1));

    let test_cases = vec![
        ("valid_short", "valid"),
        ("valid_medium", "this_is_a_valid_field_name"),
        ("valid_long", "A".repeat(100).leak() as &str),
        ("invalid_bracket", "invalid]field"),
        ("invalid_plus", "invalid+field"),
        ("invalid_brace", "invalid[field"),
    ];

    for (name, field) in test_cases {
        group.bench_function(name, |b| {
            b.iter(|| {
                let result = validate_field(black_box(field));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark field length validation for DoS protection.
///
/// Tests the performance of `validate_field_lengths()` which prevents
/// memory exhaustion by checking field size limits.
fn bench_field_lengths_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_lengths_validation");

    // Test with different numbers of fields
    for field_count in [1, 4, 10, 20].iter() {
        group.throughput(Throughput::Elements(*field_count as u64));

        // Create fields with varying lengths
        let fields: Vec<String> = (0..*field_count).map(|i| format!("field_{}", i)).collect();

        group.bench_with_input(
            BenchmarkId::new("valid_fields", field_count),
            &fields,
            |b, fields| {
                b.iter(|| {
                    let result = validate_field_lengths(black_box(fields), fields.len());
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark field length validation with oversized fields.
///
/// Tests early rejection of fields exceeding MAX_FIELD_LENGTH (256 bytes).
fn bench_field_lengths_oversized(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_lengths_oversized");
    group.throughput(Throughput::Elements(1));

    // Create fields of different sizes
    let test_cases = vec![
        ("within_limit", "A".repeat(256)),
        ("exceed_limit", "A".repeat(300)),
        ("far_exceed", "A".repeat(1000)),
    ];

    for (name, field) in test_cases {
        let fields = vec![field];
        group.bench_function(name, |b| {
            b.iter(|| {
                let result = validate_field_lengths(black_box(&fields), 1);
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark batch validation of multiple card numbers.
///
/// Simulates real-world scenario of validating access requests in bulk,
/// such as during card database import or batch processing.
fn bench_batch_card_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_card_validation");

    for batch_size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        // Create batch of valid card numbers
        let cards: Vec<&str> = (0..*batch_size)
            .map(|i| {
                // Leak strings to get 'static lifetime for benchmark
                format!("CARD{:08}", i).leak() as &str
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &cards,
            |b, cards| {
                b.iter(|| {
                    for card in cards {
                        let result = validate_card_number(black_box(card));
                        black_box(result).ok();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark complete validation pipeline.
///
/// Tests the full validation flow used when parsing access requests:
/// 1. Field delimiter validation
/// 2. Field length validation
/// 3. Card number validation
///
/// This represents real-world usage patterns.
fn bench_validation_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation_pipeline");
    group.throughput(Throughput::Elements(1));

    // Simulate access request fields
    let card = "12345678";
    let timestamp = "10/05/2025 12:46:06";
    let direction = "1";
    let reader_type = "0";

    group.bench_function("access_request_validation", |b| {
        b.iter(|| {
            // Step 1: Validate field delimiters
            validate_field(black_box(card)).ok();
            validate_field(black_box(timestamp)).ok();
            validate_field(black_box(direction)).ok();
            validate_field(black_box(reader_type)).ok();

            // Step 2: Validate field lengths
            let fields = vec![
                card.to_string(),
                timestamp.to_string(),
                direction.to_string(),
                reader_type.to_string(),
            ];
            validate_field_lengths(black_box(&fields), 4).ok();

            // Step 3: Validate card number specifically
            let result = validate_card_number(black_box(card));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark memory allocation impact comparison.
///
/// Demonstrates the performance benefit of zero-copy (&str) vs
/// allocation-based (String) validation by simulating both approaches.
fn bench_allocation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_comparison");
    group.throughput(Throughput::Elements(1));

    let card = "12345678";

    // Zero-copy approach (current implementation)
    group.bench_function("zero_copy", |b| {
        b.iter(|| {
            let result = validate_card_number(black_box(card));
            black_box(result)
        });
    });

    // Allocation-based approach (for comparison)
    // Simulates what the cost would be if we returned String instead of &str
    group.bench_function("with_allocation", |b| {
        b.iter(|| {
            // Simulate allocation-based validation
            let card_owned = black_box(card).to_string();
            // Process the validation, then clone to simulate String return
            let result = validate_card_number(&card_owned).map(|s| s.to_string());
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_card_number_validation,
    bench_card_number_scenarios,
    bench_field_validation,
    bench_field_lengths_validation,
    bench_field_lengths_oversized,
    bench_batch_card_validation,
    bench_validation_pipeline,
    bench_allocation_comparison,
);

criterion_main!(benches);
