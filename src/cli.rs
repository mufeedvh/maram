//! Command-line interface argument parsing
//!
//! This module defines all command-line arguments and options for maram
//! using the clap crate with derive macros for a clean, declarative API.

use clap::Parser;
use crate::filters::SortBy;
use crate::formatter::{OutputFormat, DistributionType, DistributionFormat};

/// maram - A modern, high-performance alternative to the Unix tree command
///
/// Recursively displays directory trees with advanced features like per-nested-path limits,
/// inline file sizes and line counts, filtering, sorting, search, and beautiful visualizations.
#[derive(Parser, Debug, Clone)]
#[command(name = "maram")]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help = false)]
pub struct Args {
    /// The directory to display (defaults to current directory)
    #[arg(default_value = ".")]
    pub path: String,
    
    // Display options
    /// Use Unicode characters for tree drawing (default: Unicode)
    #[arg(long, short = 'u')]
    pub unicode: bool,
    
    /// Enable colored output (auto-detected by default)
    #[arg(long)]
    pub color: bool,
    
    /// Disable colored output
    #[arg(long, conflicts_with = "color")]
    pub no_color: bool,
    
    /// Show full absolute paths instead of relative
    #[arg(long, short = 'f')]
    pub full_path: bool,
    
    // Per-nested-path limits
    /// Maximum number of directories to show per directory
    #[arg(long, value_name = "N")]
    pub max_dirs: Option<usize>,
    
    /// Maximum number of files to show per directory
    #[arg(long, value_name = "N")]
    pub max_files: Option<usize>,
    
    // Inline details
    /// Show file sizes (default: on)
    #[arg(long, default_value = "true")]
    pub show_size: bool,
    
    /// Show line counts for text files
    #[arg(long)]
    pub show_lines: bool,
    
    /// Show recursive directory sizes
    #[arg(long)]
    pub dir_sizes: bool,
    
    /// Maximum file size for line counting (default: 1GB)
    #[arg(long, default_value = "1073741824", value_name = "BYTES")]
    pub max_file_size: u64,
    
    // Filtering options
    /// Include only files matching this regex pattern
    #[arg(long, value_name = "PATTERN")]
    pub include: Option<String>,
    
    /// Exclude files matching this regex pattern
    #[arg(long, value_name = "PATTERN")]
    pub exclude: Option<String>,
    
    /// Show only directories
    #[arg(long, conflicts_with = "only_files")]
    pub only_dirs: bool,
    
    /// Show only files
    #[arg(long, conflicts_with = "only_dirs")]
    pub only_files: bool,
    
    /// Minimum file size to include (e.g., 1MB, 500KB)
    #[arg(long, value_name = "SIZE")]
    pub min_size: Option<String>,
    
    /// Maximum file size to include (e.g., 10MB, 1GB)
    #[arg(long, value_name = "SIZE")]
    pub max_size: Option<String>,
    
    /// Show files newer than specified time (e.g., 1d, 2h, 30m)
    #[arg(long, value_name = "TIME")]
    pub newer_than: Option<String>,
    
    /// Show files older than specified time (e.g., 1d, 2h, 30m)
    #[arg(long, value_name = "TIME")]
    pub older_than: Option<String>,
    
    /// Respect .gitignore files
    #[arg(long)]
    pub gitignore: bool,
    
    /// Show all files including hidden ones
    #[arg(short, long)]
    pub all: bool,
    
    // Sorting options
    /// Sort files by: name, size, time, ext, or lines
    #[arg(long, value_enum, value_name = "FIELD")]
    pub sort: Option<SortBy>,
    
    /// Reverse sort order
    #[arg(long, short = 'r')]
    pub reverse: bool,
    
    // Search options
    /// Search for files matching this regex pattern
    #[arg(long, value_name = "QUERY")]
    pub search: Option<String>,
    
    /// Case-insensitive search
    #[arg(long, short = 'i', requires = "search")]
    pub ignore_case: bool,
    
    // Summary options
    /// Show total size summary
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub total_size: bool,
    
    // Size distribution
    /// Show size distribution by: type, size, or ext
    #[arg(long, value_enum, value_name = "TYPE")]
    pub dist: Option<DistributionType>,
    
    /// Number of top items to show in distribution
    #[arg(long, default_value = "10", value_name = "N", requires = "dist")]
    pub top: usize,
    
    /// Distribution output format: table or chart
    #[arg(long, value_enum, default_value = "chart", value_name = "FORMAT", requires = "dist")]
    pub format: DistributionFormat,
    
    // Other options
    /// Maximum depth to traverse
    #[arg(short = 'L', long, value_name = "N")]
    pub depth: Option<usize>,
    
    /// Output format
    #[arg(long, value_enum, default_value = "tree", value_name = "FORMAT")]
    pub output: OutputFormat,
    
    /// Number of threads for parallel operations (0 = auto)
    #[arg(long, default_value = "0", value_name = "N")]
    pub threads: usize,
    
    /// Follow symbolic links
    #[arg(long)]
    pub follow_symlinks: bool,
    
    /// Show git status colors (requires git repository)
    #[arg(long)]
    pub git_status: bool,
    
    /// Enable verbose logging
    #[arg(long)]
    pub verbose: bool,
    
    /// Show timing and performance statistics
    #[arg(long)]
    pub bench: bool,
    
    /// Continue on errors instead of stopping
    #[arg(long)]
    pub ignore_errors: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            path: ".".to_string(),
            unicode: true,
            color: false,
            no_color: false,
            full_path: false,
            max_dirs: None,
            max_files: None,
            show_size: true,
            show_lines: false,
            dir_sizes: false,
            max_file_size: 1_073_741_824, // 1GB
            include: None,
            exclude: None,
            only_dirs: false,
            only_files: false,
            min_size: None,
            max_size: None,
            newer_than: None,
            older_than: None,
            gitignore: false,
            all: false,
            sort: None,
            reverse: false,
            search: None,
            ignore_case: false,
            total_size: false,
            dist: None,
            top: 10,
            format: DistributionFormat::Chart,
            depth: None,
            output: OutputFormat::Tree,
            threads: 0,
            follow_symlinks: false,
            git_status: false,
            verbose: false,
            bench: false,
            ignore_errors: false,
        }
    }
}