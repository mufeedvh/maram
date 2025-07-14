//! Benchmarks for filesystem traversal

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use maram::{FilterOptions, Walker};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Create a large directory tree for benchmarking
fn create_benchmark_tree(depth: usize, files_per_dir: usize, dirs_per_dir: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    create_tree_recursive(temp_dir.path(), depth, files_per_dir, dirs_per_dir);
    temp_dir
}

fn create_tree_recursive(path: &Path, depth: usize, files_per_dir: usize, dirs_per_dir: usize) {
    if depth == 0 {
        return;
    }
    
    // Create files
    for i in 0..files_per_dir {
        let file_path = path.join(format!("file_{}.txt", i));
        let mut file = File::create(file_path).unwrap();
        writeln!(file, "This is test file {}", i).unwrap();
    }
    
    // Create subdirectories
    for i in 0..dirs_per_dir {
        let dir_path = path.join(format!("dir_{}", i));
        fs::create_dir(&dir_path).unwrap();
        create_tree_recursive(&dir_path, depth - 1, files_per_dir, dirs_per_dir);
    }
}

fn benchmark_small_tree(c: &mut Criterion) {
    let temp_dir = create_benchmark_tree(3, 10, 3);
    let path = temp_dir.path();
    
    c.bench_function("traverse_small_tree", |b| {
        b.iter(|| {
            let filter_opts = FilterOptions::default();
            let mut walker = Walker::new(black_box(path), filter_opts, 1).unwrap();
            let _ = walker.walk().unwrap();
        });
    });
}

fn benchmark_large_tree(c: &mut Criterion) {
    let temp_dir = create_benchmark_tree(4, 20, 4);
    let path = temp_dir.path();
    
    c.bench_function("traverse_large_tree", |b| {
        b.iter(|| {
            let filter_opts = FilterOptions::default();
            let mut walker = Walker::new(black_box(path), filter_opts, 1).unwrap();
            let _ = walker.walk().unwrap();
        });
    });
}

fn benchmark_parallel_traversal(c: &mut Criterion) {
    let temp_dir = create_benchmark_tree(4, 20, 4);
    let path = temp_dir.path();
    
    c.bench_function("traverse_parallel", |b| {
        b.iter(|| {
            let filter_opts = FilterOptions::default();
            let mut walker = Walker::new(black_box(path), filter_opts, 0).unwrap(); // 0 = auto threads
            let _ = walker.walk().unwrap();
        });
    });
}

fn benchmark_filtered_traversal(c: &mut Criterion) {
    let temp_dir = create_benchmark_tree(4, 20, 4);
    let path = temp_dir.path();
    
    c.bench_function("traverse_filtered", |b| {
        b.iter(|| {
            let mut filter_opts = FilterOptions::default();
            filter_opts.include = Some(regex::Regex::new(r"file_1").unwrap());
            let mut walker = Walker::new(black_box(path), filter_opts, 1).unwrap();
            let _ = walker.walk().unwrap();
        });
    });
}

criterion_group!(
    benches,
    benchmark_small_tree,
    benchmark_large_tree,
    benchmark_parallel_traversal,
    benchmark_filtered_traversal
);
criterion_main!(benches);