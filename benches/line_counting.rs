//! Benchmarks for line counting

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use maram::stats::count_lines;
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;

fn create_file_with_lines(lines: usize) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for i in 0..lines {
        writeln!(file, "This is line number {}", i).unwrap();
    }
    file.flush().unwrap();
    file
}

fn benchmark_small_file(c: &mut Criterion) {
    let file = create_file_with_lines(100);
    
    c.bench_function("count_lines_100", |b| {
        b.iter(|| {
            let _ = count_lines(black_box(file.path()), 10_000_000).unwrap();
        });
    });
}

fn benchmark_medium_file(c: &mut Criterion) {
    let file = create_file_with_lines(10_000);
    
    c.bench_function("count_lines_10k", |b| {
        b.iter(|| {
            let _ = count_lines(black_box(file.path()), 10_000_000).unwrap();
        });
    });
}

fn benchmark_large_file(c: &mut Criterion) {
    let file = create_file_with_lines(100_000);
    
    c.bench_function("count_lines_100k", |b| {
        b.iter(|| {
            let _ = count_lines(black_box(file.path()), 100_000_000).unwrap();
        });
    });
}

fn benchmark_binary_detection(c: &mut Criterion) {
    // Create a binary file
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&[0, 1, 2, 3, 255, 254, 253]).unwrap();
    file.flush().unwrap();
    
    c.bench_function("binary_detection", |b| {
        b.iter(|| {
            let _ = count_lines(black_box(file.path()), 10_000_000).unwrap();
        });
    });
}

criterion_group!(
    benches,
    benchmark_small_file,
    benchmark_medium_file,
    benchmark_large_file,
    benchmark_binary_detection
);
criterion_main!(benches);