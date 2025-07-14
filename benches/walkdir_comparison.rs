//! Benchmarks comparing maram's custom walker with walkdir

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use maram::{FilterOptions, Walker};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;
use walkdir::WalkDir;

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

fn benchmark_maram_vs_walkdir(c: &mut Criterion) {
    let temp_dir = create_benchmark_tree(4, 10, 3);
    let path = temp_dir.path();
    
    let mut group = c.benchmark_group("walker_comparison");
    
    // Benchmark walkdir
    group.bench_function("walkdir", |b| {
        b.iter(|| {
            let walker = WalkDir::new(black_box(path));
            let count = walker.into_iter().filter_map(|e| e.ok()).count();
            black_box(count);
        });
    });
    
    // Benchmark maram (single-threaded)
    group.bench_function("maram_single", |b| {
        b.iter(|| {
            let filter_opts = FilterOptions::default();
            let mut walker = Walker::new(black_box(path), filter_opts, 1).unwrap();
            let entries = walker.walk().unwrap();
            black_box(entries.len());
        });
    });
    
    // Benchmark maram (multi-threaded)
    group.bench_function("maram_parallel", |b| {
        b.iter(|| {
            let filter_opts = FilterOptions::default();
            let mut walker = Walker::new(black_box(path), filter_opts, 0).unwrap();
            let entries = walker.walk().unwrap();
            black_box(entries.len());
        });
    });
    
    group.finish();
}

fn benchmark_with_filtering(c: &mut Criterion) {
    let temp_dir = create_benchmark_tree(4, 20, 4);
    let path = temp_dir.path();
    
    let mut group = c.benchmark_group("filtered_walker_comparison");
    
    // Benchmark walkdir with filtering
    group.bench_function("walkdir_filtered", |b| {
        b.iter(|| {
            let walker = WalkDir::new(black_box(path));
            let count = walker
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .contains("file_1")
                })
                .count();
            black_box(count);
        });
    });
    
    // Benchmark maram with filtering
    group.bench_function("maram_filtered", |b| {
        b.iter(|| {
            let mut filter_opts = FilterOptions::default();
            filter_opts.include = Some(regex::Regex::new(r"file_1").unwrap());
            let mut walker = Walker::new(black_box(path), filter_opts, 1).unwrap();
            let entries = walker.walk().unwrap();
            black_box(entries.len());
        });
    });
    
    group.finish();
}

fn benchmark_large_tree(c: &mut Criterion) {
    // Create a larger tree for more realistic benchmarks
    let temp_dir = create_benchmark_tree(5, 20, 3);
    let path = temp_dir.path();
    
    let mut group = c.benchmark_group("large_tree");
    group.sample_size(10); // Reduce sample size for large trees
    
    group.bench_function("walkdir_large", |b| {
        b.iter(|| {
            let walker = WalkDir::new(black_box(path));
            let count = walker.into_iter().filter_map(|e| e.ok()).count();
            black_box(count);
        });
    });
    
    group.bench_function("maram_large_single", |b| {
        b.iter(|| {
            let filter_opts = FilterOptions::default();
            let mut walker = Walker::new(black_box(path), filter_opts, 1).unwrap();
            let entries = walker.walk().unwrap();
            black_box(entries.len());
        });
    });
    
    group.bench_function("maram_large_parallel", |b| {
        b.iter(|| {
            let filter_opts = FilterOptions::default();
            let mut walker = Walker::new(black_box(path), filter_opts, 0).unwrap();
            let entries = walker.walk().unwrap();
            black_box(entries.len());
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_maram_vs_walkdir,
    benchmark_with_filtering,
    benchmark_large_tree
);
criterion_main!(benches);