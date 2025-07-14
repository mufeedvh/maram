//! Filtering, sorting, and search functionality
//!
//! This module provides all the logic for filtering files based on various criteria,
//! sorting entries, and searching through the tree structure.

use crate::{Args, Config, Error, Result};
use clap::ValueEnum;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Sorting criteria for tree entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    /// Sort by name (alphabetical)
    Name,
    /// Sort by file size
    Size,
    /// Sort by modification time
    Time,
    /// Sort by file extension
    Ext,
    /// Sort by line count (only for files)
    Lines,
}

/// Options for filtering directory entries
#[derive(Debug, Clone)]
pub struct FilterOptions {
    /// Include pattern (regex)
    pub include: Option<Regex>,
    /// Exclude pattern (regex)
    pub exclude: Option<Regex>,
    /// Show only directories
    pub only_dirs: bool,
    /// Show only files
    pub only_files: bool,
    /// Minimum file size in bytes
    pub min_size: Option<u64>,
    /// Maximum file size in bytes
    pub max_size: Option<u64>,
    /// Files newer than this duration
    pub newer_than: Option<Duration>,
    /// Files older than this duration
    pub older_than: Option<Duration>,
    /// Respect gitignore files
    pub gitignore: bool,
    /// Show hidden files
    pub show_hidden: bool,
    /// Search pattern (regex)
    pub search: Option<Regex>,
    /// Maximum depth to traverse
    pub max_depth: Option<usize>,
    /// Maximum directories per level
    pub max_dirs: Option<usize>,
    /// Maximum files per level
    pub max_files: Option<usize>,
    /// Sort criteria
    pub sort_by: Option<SortBy>,
    /// Reverse sort order
    pub reverse_sort: bool,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            include: None,
            exclude: None,
            only_dirs: false,
            only_files: false,
            min_size: None,
            max_size: None,
            newer_than: None,
            older_than: None,
            gitignore: false,
            show_hidden: false,
            search: None,
            max_depth: None,
            max_dirs: None,
            max_files: None,
            sort_by: None,
            reverse_sort: false,
        }
    }
}

impl FilterOptions {
    /// Create filter options from command line arguments and config
    pub fn from_args_and_config(args: &Args, config: &Config) -> Result<Self> {
        let mut opts = Self {
            include: None,
            exclude: None,
            only_dirs: args.only_dirs,
            only_files: args.only_files,
            min_size: None,
            max_size: None,
            newer_than: None,
            older_than: None,
            gitignore: args.gitignore || config.filters.gitignore,
            show_hidden: args.all || config.filters.show_hidden,
            search: None,
            max_depth: args.depth.or(config.filters.max_depth),
            max_dirs: args.max_dirs.or(config.filters.max_dirs),
            max_files: args.max_files.or(config.filters.max_files),
            sort_by: args.sort.or(config.filters.sort_by),
            reverse_sort: args.reverse || config.filters.reverse_sort,
        };
        
        // Compile regex patterns
        if let Some(pattern) = &args.include {
            opts.include = Some(compile_regex(pattern, args.ignore_case)?);
        }
        
        if let Some(pattern) = &args.exclude {
            opts.exclude = Some(compile_regex(pattern, args.ignore_case)?);
        }
        
        if let Some(pattern) = &args.search {
            opts.search = Some(compile_regex(pattern, args.ignore_case)?);
        }
        
        // Parse size filters
        if let Some(size_str) = &args.min_size {
            opts.min_size = Some(parse_size(size_str)?);
        }
        
        if let Some(size_str) = &args.max_size {
            opts.max_size = Some(parse_size(size_str)?);
        }
        
        // Parse time filters
        if let Some(time_str) = &args.newer_than {
            opts.newer_than = Some(parse_duration(time_str)?);
        }
        
        if let Some(time_str) = &args.older_than {
            opts.older_than = Some(parse_duration(time_str)?);
        }
        
        Ok(opts)
    }
    
    /// Check if a path should be included based on filters
    pub fn should_include(&self, path: &Path, metadata: &std::fs::Metadata) -> bool {
        // Check if it's a directory or file
        let is_dir = metadata.is_dir();
        if self.only_dirs && !is_dir {
            return false;
        }
        if self.only_files && is_dir {
            return false;
        }
        
        // Check hidden files
        if !self.show_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    return false;
                }
            }
        }
        
        // Check search pattern (only for files, not directories)
        if !is_dir && self.search.is_some() && !self.matches_search(path) {
            return false;
        }
        
        // Check include pattern
        if let Some(regex) = &self.include {
            let path_str = path.to_string_lossy();
            if !regex.is_match(&path_str) {
                return false;
            }
        }
        
        // Check exclude pattern
        if let Some(regex) = &self.exclude {
            let path_str = path.to_string_lossy();
            if regex.is_match(&path_str) {
                return false;
            }
        }
        
        // Check size filters (only for files)
        if !is_dir {
            let size = metadata.len();
            if let Some(min) = self.min_size {
                if size < min {
                    return false;
                }
            }
            if let Some(max) = self.max_size {
                if size > max {
                    return false;
                }
            }
        }
        
        // Check time filters
        if let Ok(modified) = metadata.modified() {
            let now = SystemTime::now();
            if let Ok(age) = now.duration_since(modified) {
                if let Some(newer_than) = self.newer_than {
                    if age > newer_than {
                        return false;
                    }
                }
                if let Some(older_than) = self.older_than {
                    if age < older_than {
                        return false;
                    }
                }
            }
        }
        
        true
    }
    
    /// Check if a path matches the search pattern
    pub fn matches_search(&self, path: &Path) -> bool {
        if let Some(regex) = &self.search {
            let path_str = path.to_string_lossy();
            regex.is_match(&path_str)
        } else {
            true
        }
    }
}

/// Compile a regex pattern with optional case insensitivity
fn compile_regex(pattern: &str, ignore_case: bool) -> Result<Regex> {
    let mut builder = regex::RegexBuilder::new(pattern);
    if ignore_case {
        builder.case_insensitive(true);
    }
    builder.build().map_err(Into::into)
}

/// Parse a human-readable size string (e.g., "1MB", "500KB") into bytes
fn parse_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim().to_uppercase();
    
    // Extract number and unit
    let (num_str, unit) = if let Some(pos) = size_str.find(|c: char| c.is_alphabetic()) {
        size_str.split_at(pos)
    } else {
        (size_str.as_str(), "")
    };
    
    // Parse the number
    let num: f64 = num_str.trim().parse()
        .map_err(|_| Error::size_parse(format!("Invalid number: {}", num_str)))?;
    
    // Convert to bytes based on unit
    let bytes = match unit.trim() {
        "" | "B" => num,
        "K" | "KB" => num * 1024.0,
        "M" | "MB" => num * 1024.0 * 1024.0,
        "G" | "GB" => num * 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" => num * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return Err(Error::size_parse(format!("Unknown size unit: {}", unit))),
    };
    
    Ok(bytes as u64)
}

/// Parse a human-readable duration string (e.g., "1d", "2h", "30m") into a Duration
fn parse_duration(time_str: &str) -> Result<Duration> {
    let time_str = time_str.trim().to_lowercase();
    
    // Extract number and unit
    let (num_str, unit) = if let Some(pos) = time_str.find(|c: char| c.is_alphabetic()) {
        time_str.split_at(pos)
    } else {
        return Err(Error::time_parse("No time unit specified"));
    };
    
    // Parse the number
    let num: u64 = num_str.trim().parse()
        .map_err(|_| Error::time_parse(format!("Invalid number: {}", num_str)))?;
    
    // Convert to seconds based on unit
    let seconds = match unit.trim() {
        "s" | "sec" | "second" | "seconds" => num,
        "m" | "min" | "minute" | "minutes" => num * 60,
        "h" | "hr" | "hour" | "hours" => num * 60 * 60,
        "d" | "day" | "days" => num * 60 * 60 * 24,
        "w" | "week" | "weeks" => num * 60 * 60 * 24 * 7,
        _ => return Err(Error::time_parse(format!("Unknown time unit: {}", unit))),
    };
    
    Ok(Duration::from_secs(seconds))
}

/// Comparator for sorting entries
pub fn compare_entries(
    a: &crate::walker::TreeEntry,
    b: &crate::walker::TreeEntry,
    sort_by: SortBy,
    reverse: bool,
) -> Ordering {
    let ordering = match sort_by {
        SortBy::Name => a.name.cmp(&b.name),
        SortBy::Size => a.size.cmp(&b.size),
        SortBy::Time => a.modified.cmp(&b.modified),
        SortBy::Ext => {
            let ext_a = Path::new(&a.name).extension().unwrap_or_default();
            let ext_b = Path::new(&b.name).extension().unwrap_or_default();
            ext_a.cmp(&ext_b).then_with(|| a.name.cmp(&b.name))
        }
        SortBy::Lines => a.line_count.cmp(&b.line_count),
    };
    
    if reverse {
        ordering.reverse()
    } else {
        ordering
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("5MB").unwrap(), 5 * 1024 * 1024);
        assert_eq!(parse_size("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
    }
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(5 * 60));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(2 * 60 * 60));
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(24 * 60 * 60));
    }
}