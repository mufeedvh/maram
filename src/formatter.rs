//! Output formatting and visualization
//!
//! This module handles all output formatting including tree visualization,
//! JSON/CSV export, and beautiful size distribution charts.

use crate::{Args, Config, Result, TreeEntry, TreeStats};
use clap::ValueEnum;
use colored::*;
use serde_json;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Tree visualization (default)
    Tree,
    /// JSON output
    Json,
    /// CSV output
    Csv,
    /// Plain text list
    Plain,
}

/// Size distribution types
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DistributionType {
    /// Distribution by file type
    Type,
    /// Distribution by size buckets
    Size,
    /// Distribution by file extension
    Ext,
}

/// Distribution output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DistributionFormat {
    /// Table format
    Table,
    /// Bar chart format
    Chart,
}

/// Options for formatting output
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Use Unicode characters for tree
    pub unicode: bool,
    /// Use colored output
    pub color: bool,
    /// Show full paths
    pub full_path: bool,
    /// Show file sizes
    pub show_size: bool,
    /// Show line counts
    pub show_lines: bool,
    /// Show directory sizes
    pub dir_sizes: bool,
}

impl FormatOptions {
    /// Create format options from args and config
    pub fn from_args_and_config(args: &Args, config: &Config) -> Self {
        let color = if args.no_color {
            false
        } else if args.color {
            true
        } else {
            // Auto-detect based on terminal
            atty::is(atty::Stream::Stdout) && std::env::var("NO_COLOR").is_err()
        };
        
        Self {
            unicode: args.unicode || config.display.unicode,
            color,
            full_path: args.full_path,
            show_size: args.show_size,
            show_lines: args.show_lines,
            dir_sizes: args.dir_sizes,
        }
    }
}

/// Tree drawing characters
struct TreeChars {
    down: &'static str,
    down_right: &'static str,
    last: &'static str,
}

impl TreeChars {
    fn new(unicode: bool) -> Self {
        if unicode {
            Self {
                down: "│   ",
                down_right: "├── ",
                last: "└── ",
            }
        } else {
            Self {
                down: "│   ",
                down_right: "├── ",
                last: "└── ",
            }
        }
    }
}

/// Print tree visualization
pub fn print_tree(entries: &[TreeEntry], opts: &FormatOptions) -> Result<()> {
    let chars = TreeChars::new(opts.unicode);
    let mut stdout = io::stdout();
    
    for (i, entry) in entries.iter().enumerate() {
        print_tree_entry(&mut stdout, entry, &chars, opts, Vec::new(), i == entries.len() - 1)?;
    }
    
    // Print summary line like tree command
    let stats = TreeStats::from_entries(entries);
    println!();
    println!("{} {}, {} {}",
        stats.dir_count,
        if stats.dir_count == 1 { "directory" } else { "directories" },
        stats.file_count,
        if stats.file_count == 1 { "file" } else { "files" }
    );
    
    Ok(())
}

/// Print a single tree entry recursively
fn print_tree_entry(
    out: &mut dyn Write,
    entry: &TreeEntry,
    chars: &TreeChars,
    opts: &FormatOptions,
    prefix: Vec<bool>,
    is_last: bool,
) -> Result<()> {
    // Print prefix
    for &cont in &prefix {
        write!(out, "{}", if cont { chars.down } else { "    " })?;
    }
    
    // Print connector
    write!(out, "{}", if is_last { chars.last } else { chars.down_right })?;
    
    // Format name with color
    let name = if opts.color {
        if entry.is_dir {
            entry.name.blue().bold().to_string()
        } else if entry.is_symlink {
            entry.name.cyan().to_string()
        } else if entry.is_executable {
            entry.name.green().to_string()
        } else {
            entry.name.clone()
        }
    } else {
        entry.name.clone()
    };
    
    // Add details
    let mut details = Vec::new();
    
    if opts.show_size && (!entry.is_dir || opts.dir_sizes) {
        details.push(format_size(entry.size));
    }
    
    if opts.show_lines && entry.line_count > 0 {
        details.push(format!("{} lines", entry.line_count));
    }
    
    // Print entry
    if details.is_empty() {
        writeln!(out, "{}", name)?;
    } else {
        let detail_str = if opts.color {
            format!(" ({})", details.join(", ")).dimmed().to_string()
        } else {
            format!(" ({})", details.join(", "))
        };
        writeln!(out, "{}{}", name, detail_str)?;
    }
    
    // Print children
    if !entry.children.is_empty() {
        let mut new_prefix = prefix;
        new_prefix.push(!is_last);
        
        for (i, child) in entry.children.iter().enumerate() {
            print_tree_entry(
                out,
                child,
                chars,
                opts,
                new_prefix.clone(),
                i == entry.children.len() - 1,
            )?;
        }
    }
    
    Ok(())
}

/// Print JSON output
pub fn print_json(entries: &[TreeEntry]) -> Result<()> {
    let json = serde_json::to_string_pretty(entries)?;
    println!("{}", json);
    Ok(())
}

/// Print CSV output
pub fn print_csv(entries: &[TreeEntry]) -> Result<()> {
    println!("path,type,size,lines,modified");
    
    fn print_csv_entry(entry: &TreeEntry, parent_path: &str) -> Result<()> {
        let path = if parent_path.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", parent_path, entry.name)
        };
        
        let entry_type = if entry.is_dir { "directory" } else { "file" };
        let modified = entry.modified.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        println!("{},{},{},{},{}", path, entry_type, entry.size, entry.line_count, modified);
        
        for child in &entry.children {
            print_csv_entry(child, &path)?;
        }
        
        Ok(())
    }
    
    for entry in entries {
        print_csv_entry(entry, "")?;
    }
    
    Ok(())
}

/// Print plain text output
pub fn print_plain(entries: &[TreeEntry]) -> Result<()> {
    fn print_plain_entry(entry: &TreeEntry, depth: usize) -> Result<()> {
        println!("{}{}", "  ".repeat(depth), entry.name);
        
        for child in &entry.children {
            print_plain_entry(child, depth + 1)?;
        }
        
        Ok(())
    }
    
    for entry in entries {
        print_plain_entry(entry, 0)?;
    }
    
    Ok(())
}

/// Print total size summary
pub fn print_total_size(stats: &TreeStats, opts: &FormatOptions) -> Result<()> {
    let total_str = format!(
        "\nTotal: {} ({} files: {}, {} directories: {})",
        format_size(stats.total_size),
        stats.file_count,
        format_size(stats.file_size),
        stats.dir_count,
        format_size(stats.dir_size),
    );
    
    if opts.color {
        println!("{}", total_str.bright_yellow().bold());
    } else {
        println!("{}", total_str);
    }
    
    Ok(())
}

/// Print size distribution
pub fn print_distribution(
    entries: &[TreeEntry],
    dist_type: &DistributionType,
    top: usize,
    format: &DistributionFormat,
    opts: &FormatOptions,
) -> Result<()> {
    let distribution = calculate_distribution(entries, dist_type);
    
    // Sort by size descending and take top N
    let mut sorted: Vec<_> = distribution.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(top);
    
    // Calculate total for percentages
    let total: u64 = sorted.iter().map(|(_, size)| size).sum();
    
    match format {
        DistributionFormat::Table => print_distribution_table(&sorted, total, opts),
        DistributionFormat::Chart => print_distribution_chart(&sorted, total, opts),
    }
}

/// Calculate size distribution
fn calculate_distribution(
    entries: &[TreeEntry],
    dist_type: &DistributionType,
) -> HashMap<String, u64> {
    let mut dist = HashMap::new();
    
    fn process_entry(
        entry: &TreeEntry,
        dist: &mut HashMap<String, u64>,
        dist_type: &DistributionType,
    ) {
        if !entry.is_dir {
            let key = match dist_type {
                DistributionType::Type => {
                    // Determine file type by extension
                    let ext = Path::new(&entry.name)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("no extension");
                    
                    match ext.to_lowercase().as_str() {
                        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => "Images",
                        "mp4" | "avi" | "mkv" | "mov" | "wmv" => "Videos",
                        "mp3" | "wav" | "flac" | "aac" | "ogg" => "Audio",
                        "zip" | "tar" | "gz" | "7z" | "rar" => "Archives",
                        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "Documents",
                        "rs" | "js" | "ts" | "py" | "go" | "c" | "cpp" | "java" => "Code",
                        "txt" | "md" | "log" => "Text",
                        _ => "Other",
                    }.to_string()
                }
                DistributionType::Size => {
                    // Size buckets
                    match entry.size {
                        0..=1024 => "< 1KB",
                        1025..=1_048_576 => "1KB - 1MB",
                        1_048_577..=10_485_760 => "1MB - 10MB",
                        10_485_761..=104_857_600 => "10MB - 100MB",
                        104_857_601..=1_073_741_824 => "100MB - 1GB",
                        _ => "> 1GB",
                    }.to_string()
                }
                DistributionType::Ext => {
                    // By extension
                    Path::new(&entry.name)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("no extension")
                        .to_string()
                }
            };
            
            *dist.entry(key).or_insert(0) += entry.size;
        }
        
        for child in &entry.children {
            process_entry(child, dist, dist_type);
        }
    }
    
    for entry in entries {
        process_entry(entry, &mut dist, dist_type);
    }
    
    dist
}

/// Print distribution as a table
fn print_distribution_table(
    data: &[(String, u64)],
    total: u64,
    opts: &FormatOptions,
) -> Result<()> {
    println!("\n{:>15} {:>12} {:>8}", "Category", "Size", "Percent");
    println!("{}", "-".repeat(40));
    
    for (category, size) in data {
        let percent = (*size as f64 / total as f64) * 100.0;
        let line = format!(
            "{:>15} {:>12} {:>7.1}%",
            category,
            format_size(*size),
            percent
        );
        
        if opts.color {
            println!("{}", line.bright_white());
        } else {
            println!("{}", line);
        }
    }
    
    println!("{}", "-".repeat(40));
    println!("{:>15} {:>12} {:>7.1}%", "Total", format_size(total), 100.0);
    
    Ok(())
}

/// Print distribution as a beautiful bar chart
fn print_distribution_chart(
    data: &[(String, u64)],
    total: u64,
    opts: &FormatOptions,
) -> Result<()> {
    println!("\n{}", "Size Distribution".bold());
    println!();
    
    // Calculate max width for bars (terminal width - space for labels)
    let term_width = terminal_width().saturating_sub(35);
    let bar_char = if opts.unicode { "█" } else { "#" };
    let empty_char = if opts.unicode { "░" } else { "-" };
    
    for (category, size) in data {
        let percent = (*size as f64 / total as f64) * 100.0;
        let bar_width = ((percent / 100.0) * term_width as f64) as usize;
        let empty_width = term_width.saturating_sub(bar_width);
        
        // Format label
        let label = format!("{:>12}", category);
        let percent_str = format!("{:>5.1}%", percent);
        let size_str = format_size(*size);
        
        // Create bar
        let bar = bar_char.repeat(bar_width);
        let empty = empty_char.repeat(empty_width);
        
        // Color based on size
        let (label_color, bar_color) = if opts.color {
            match percent as u32 {
                0..=10 => (label.green(), bar.green()),
                11..=25 => (label.yellow(), bar.yellow()),
                26..=50 => (label.bright_yellow(), bar.bright_yellow()),
                _ => (label.red(), bar.red()),
            }
        } else {
            (label.normal(), bar.normal())
        };
        
        println!(
            "{} {} [{}{}] {}",
            label_color,
            percent_str.dimmed(),
            bar_color,
            empty.dimmed(),
            size_str.bright_white()
        );
    }
    
    println!("\n{:>12} {:>6} {} {}", 
        "Total".bold(), 
        "100.0%".dimmed(),
        " ".repeat(term_width + 2),
        format_size(total).bright_white().bold()
    );
    
    Ok(())
}

/// Format size in human-readable format
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;
    
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    
    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

/// Get terminal width
fn terminal_width() -> usize {
    // Try to get terminal width, default to 80 if unavailable
    term_size::dimensions().map(|(w, _)| w).unwrap_or(80)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
    }
}