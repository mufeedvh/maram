//! maram - CLI entry point
//!
//! This module contains the main function that serves as the entry point
//! for the maram command-line tool. It handles argument parsing, configuration
//! loading, logging setup, and delegates to the core library functions.

#[cfg(feature = "jemalloc")]
use tikv_jemallocator::Jemalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use clap::Parser;
use env_logger::Env;
use maram::{run_tree, Args, Config, Result};
use std::error::Error;
use std::path::Path;
use std::process;

fn main() {
    // Initialize logger with RUST_LOG env var support
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    
    // Run the main logic
    if let Err(e) = run() {
        log::error!("Error: {}", e);
        eprintln!("Error: {}", e);
        
        // Print chain of errors if any
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("  Caused by: {}", err);
            source = err.source();
        }
        
        process::exit(1);
    }
}

/// Main application logic
///
/// This function handles configuration loading, validates arguments,
/// and calls the tree traversal and display functions.
fn run() -> Result<()> {
    let args = Args::parse();
    
    // Load configuration from ~/.maram.toml if it exists
    let config = Config::load()?;
    
    // Get the target path
    let path = Path::new(&args.path);
    
    // Validate path exists
    if !path.exists() {
        return Err(maram::Error::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path does not exist: {}", path.display()),
        )));
    }
    
    // Enable verbose logging if requested
    if args.verbose {
        log::set_max_level(log::LevelFilter::Debug);
    }
    
    // Run the tree command
    let start = std::time::Instant::now();
    run_tree(path, &args, &config)?;
    
    // Show timing info if benchmarking
    if args.bench {
        let elapsed = start.elapsed();
        eprintln!("\nExecution time: {:.3}s", elapsed.as_secs_f64());
    }
    
    Ok(())
}
