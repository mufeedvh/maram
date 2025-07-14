//! File statistics and line counting
//!
//! This module provides high-performance implementations for counting lines
//! in files and calculating directory statistics. All implementations are
//! optimized for speed and use parallelism where beneficial.

use crate::{Result, TreeEntry};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Statistics for a file or directory
#[derive(Debug, Clone, Default)]
pub struct FileStats {
    /// Size in bytes
    pub size: u64,
    /// Number of lines (0 for directories and binary files)
    pub line_count: u64,
    /// Is this a directory?
    pub is_dir: bool,
}

/// Aggregated statistics for a tree
#[derive(Debug, Clone, Default)]
pub struct TreeStats {
    /// Total size of all files and directories
    pub total_size: u64,
    /// Total size of files only
    pub file_size: u64,
    /// Total size of directories only
    pub dir_size: u64,
    /// Number of files
    pub file_count: usize,
    /// Number of directories
    pub dir_count: usize,
    /// Total line count across all text files
    pub total_lines: u64,
}

impl TreeStats {
    /// Calculate statistics from a slice of tree entries
    pub fn from_entries(entries: &[TreeEntry]) -> Self {
        let mut stats = Self::default();
        
        for entry in entries {
            stats.add_entry(entry);
        }
        
        stats
    }
    
    /// Add statistics from a tree entry recursively
    fn add_entry(&mut self, entry: &TreeEntry) {
        self.total_size += entry.size;
        
        if entry.is_dir {
            self.dir_count += 1;
            self.dir_size += entry.size;
        } else {
            self.file_count += 1;
            self.file_size += entry.size;
            self.total_lines += entry.line_count;
        }
        
        // Recursively process children
        for child in &entry.children {
            self.add_entry(child);
        }
    }
}

/// Count lines in a file - optimized implementation
///
/// This function reads files in 4KB chunks and counts newline characters.
/// It skips binary files and files larger than max_size for performance.
pub fn count_lines(path: &Path, max_size: u64) -> Result<u64> {
    // Check if file is too large
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > max_size {
        log::debug!("Skipping line count for large file: {:?}", path);
        return Ok(0);
    }
    
    // Check if file is likely binary by reading first few bytes
    if is_binary_file(path)? {
        log::debug!("Skipping line count for binary file: {:?}", path);
        return Ok(0);
    }
    
    // Open file and count lines
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(4096, file);
    let mut count = 0u64;
    let mut buffer = [0u8; 4096];
    let mut last_byte = 0u8;
    let mut has_content = false;
    
    loop {
        match reader.read(&mut buffer)? {
            0 => break,
            n => {
                has_content = true;
                count += buffer[..n].iter().filter(|&&b| b == b'\n').count() as u64;
                last_byte = buffer[n - 1];
            }
        }
    }
    
    // If file has content and doesn't end with newline, add 1 to count
    if has_content && last_byte != b'\n' {
        count += 1;
    }
    
    Ok(count)
}

/// Count lines in multiple files in parallel
pub fn count_lines_parallel(paths: &[&Path], max_size: u64) -> Vec<u64> {
    paths
        .par_iter()
        .map(|path| count_lines(path, max_size).unwrap_or(0))
        .collect()
}

/// Check if a file is likely binary by examining first bytes
fn is_binary_file(path: &Path) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 512];
    
    let bytes_read = file.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(false);
    }
    
    // Check for null bytes or high percentage of non-text characters
    let null_count = buffer[..bytes_read].iter().filter(|&&b| b == 0).count();
    if null_count > 0 {
        return Ok(true);
    }
    
    // Count printable ASCII and common UTF-8 characters
    let text_chars = buffer[..bytes_read]
        .iter()
        .filter(|&&b| {
            b == b'\n' || b == b'\r' || b == b'\t' || (b >= 32 && b <= 126) || b >= 128
        })
        .count();
    
    // If less than 95% are text characters, consider it binary
    Ok(text_chars < (bytes_read * 95) / 100)
}

/// Calculate directory size recursively using parallel processing
pub fn calculate_dir_size(path: &Path) -> Result<u64> {
    let size = Arc::new(AtomicU64::new(0));
    
    calculate_dir_size_recursive(path, &size)?;
    
    Ok(size.load(Ordering::Relaxed))
}

/// Recursive helper for directory size calculation
fn calculate_dir_size_recursive(path: &Path, size: &Arc<AtomicU64>) -> Result<()> {
    let entries = std::fs::read_dir(path)?;
    
    // Collect entries to process in parallel
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    
    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        if metadata.is_dir() {
            dirs.push(entry.path());
        } else {
            files.push(metadata.len());
        }
    }
    
    // Add file sizes
    let file_sum: u64 = files.iter().sum();
    size.fetch_add(file_sum, Ordering::Relaxed);
    
    // Process subdirectories in parallel
    dirs.par_iter().try_for_each(|dir| {
        calculate_dir_size_recursive(dir, size)
    })?;
    
    Ok(())
}

/// Format a duration in human-readable format
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_count_lines() {
        // Create temporary file with known content
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        write!(file, "line 3").unwrap(); // No trailing newline
        file.flush().unwrap();
        
        let count = count_lines(file.path(), 1_000_000).unwrap();
        assert_eq!(count, 3);
    }
    
    #[test]
    fn test_is_binary_file() {
        // Test text file
        let mut text_file = NamedTempFile::new().unwrap();
        writeln!(text_file, "This is a text file").unwrap();
        assert!(!is_binary_file(text_file.path()).unwrap());
        
        // Test binary file
        let mut bin_file = NamedTempFile::new().unwrap();
        bin_file.write_all(&[0, 1, 2, 3, 255, 254]).unwrap();
        assert!(is_binary_file(bin_file.path()).unwrap());
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3725), "1h 2m");
    }
}