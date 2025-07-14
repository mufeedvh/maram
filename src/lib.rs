//! maram - A modern, high-performance alternative to the Unix tree command
//!
//! This crate provides a fast and feature-rich directory tree visualization tool
//! with advanced features like per-nested-path limits, inline file sizes and line counts,
//! filtering, sorting, search, and beautiful size distribution visualizations.
//!
//! # Features
//!
//! - **Fast filesystem traversal**: Custom iterative walker with parallelism
//! - **Rich filtering**: Regex patterns, size ranges, time-based filters
//! - **Beautiful output**: ASCII/Unicode tree with colors and inline details
//! - **Size distribution**: Visual charts showing disk usage by type/extension
//! - **Line counting**: Fast parallel line counting for text files
//! - **Configuration**: Support for .maram.toml config files
//! - **Cross-platform**: Works on Linux, macOS, and Windows

pub mod cli;
pub mod config;
pub mod error;
pub mod filters;
pub mod formatter;
pub mod stats;
pub mod walker;

pub use cli::Args;
pub use config::Config;
pub use error::{Error, Result};
pub use filters::{FilterOptions, SortBy};
pub use formatter::{FormatOptions, OutputFormat};
pub use stats::{FileStats, TreeStats};
pub use walker::{TreeEntry, Walker};

use std::path::Path;

/// Main entry point for the maram tree visualization
///
/// # Arguments
///
/// * `path` - The root path to start traversing from
/// * `args` - Command line arguments parsed by clap
/// * `config` - Configuration loaded from .maram.toml (if exists)
///
/// # Returns
///
/// Returns Ok(()) on success, or an Error if something goes wrong
///
/// # Example
///
/// ```no_run
/// use maram::{run_tree, Args, Config};
/// use std::path::Path;
///
/// let args = Args::default();
/// let config = Config::default();
/// run_tree(Path::new("."), &args, &config).unwrap();
/// ```
pub fn run_tree(path: &Path, args: &Args, config: &Config) -> Result<()> {
    log::debug!("Starting tree traversal at: {:?}", path);
    
    // Merge CLI args with config to get final options first
    let filter_opts = FilterOptions::from_args_and_config(args, config)?;
    let format_opts = FormatOptions::from_args_and_config(args, config);
    
    // Check if we need buffered mode for advanced features
    let needs_buffering = matches!(args.output, OutputFormat::Json | OutputFormat::Csv)
        || args.dist.is_some()           // Distribution analysis
        || args.total_size                // Total size calculation
        || args.dir_sizes                 // Directory size calculation
        || filter_opts.sort_by.is_some(); // Sorting required
    
    // Use streaming by default for better performance
    if !needs_buffering {
        log::debug!("Using streaming output for {:?} format", args.output);
        
        // Apply config defaults properly
        // Unicode defaults to true from config, use args.unicode only if explicitly set
        let unicode = args.unicode || config.display.unicode;
        let show_size = args.show_size || config.display.show_size;
        let show_lines = args.show_lines || config.display.show_lines;
        
        // Use streaming walker for direct output
        let mut stream_walker = walker::StreamWalker::new(
            filter_opts,
            args.output,
            show_size,
            show_lines,
            unicode,
        );
        
        return stream_walker.stream(path);
    }
    
    
    // Buffered path for features that need the full tree
    log::debug!("Using buffered walker for advanced features");
    
    // Create walker with options
    let mut walker = Walker::new(path, filter_opts, args.threads)?;
    
    // Configure walker based on args
    walker.set_max_file_size(args.max_file_size);
    if args.show_lines {
        walker.enable_line_counting();
    }
    if args.dir_sizes {
        walker.enable_dir_sizes();
    }
    
    // Perform traversal
    let entries = walker.walk()?;
    
    // Format and output results
    match args.output {
        OutputFormat::Tree => formatter::print_tree(&entries, &format_opts)?,
        OutputFormat::Json => formatter::print_json(&entries)?,
        OutputFormat::Csv => formatter::print_csv(&entries)?,
        OutputFormat::Plain => formatter::print_plain(&entries)?,
    }
    
    // Show total size if requested
    if args.total_size && matches!(args.output, OutputFormat::Tree) {
        let stats = TreeStats::from_entries(&entries);
        formatter::print_total_size(&stats, &format_opts)?;
    }
    
    // Show size distribution if requested
    if let Some(dist_type) = &args.dist {
        formatter::print_distribution(&entries, dist_type, args.top, &args.format, &format_opts)?;
    }
    
    Ok(())
}