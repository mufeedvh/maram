//! Unified high-performance filesystem walker
//!
//! This module provides a single, optimized filesystem walker that automatically
//! chooses the fastest implementation based on platform capabilities and requested
//! features. It supports all features consistently across all platforms.
//!
//! # Implementation Strategy
//!
//! - **Unix systems**: Uses direct syscalls for maximum performance when possible
//! - **Windows**: Uses optimized Win32 APIs where available
//! - **Fallback**: Uses std::fs for compatibility
//! - **Feature detection**: Automatically falls back to slower paths when advanced
//!   features are requested that require more processing

use crate::{FilterOptions, Result, Error};
use crate::formatter::OutputFormat as FormatterOutputFormat;
use crate::filters::compare_entries;
use crate::stats::{calculate_dir_size, count_lines};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, Metadata};
use std::io::{self, Write, BufWriter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use colored::*;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use libc;
#[cfg(unix)]
use std::ffi::{CStr, CString, OsStr};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use winapi::um::fileapi::{FindFirstFileW, FindNextFileW, FindClose};
#[cfg(windows)]
use winapi::um::minwinbase::WIN32_FIND_DATAW;

/// A tree entry representing a file or directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Entry name (not full path)
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Number of lines (0 for directories and binary files)
    pub line_count: u64,
    /// Modification time
    pub modified: SystemTime,
    /// Is this a directory?
    pub is_dir: bool,
    /// Is this a symlink?
    pub is_symlink: bool,
    /// Is this executable?
    pub is_executable: bool,
    /// Child entries
    pub children: Vec<TreeEntry>,
    /// Depth from root
    pub depth: usize,
}

/// Walker mode based on features requested
#[derive(Debug, Clone, Copy)]
enum WalkerMode {
    /// Ultra-fast mode using syscalls, minimal features
    FastPath,
    /// Standard mode with filtering and moderate features
    Standard,
    /// Full mode with all features enabled
    Full,
}

/// Unified walker that automatically chooses optimal implementation
pub struct Walker {
    root: PathBuf,
    filter_opts: FilterOptions,
    gitignore: Option<Gitignore>,
    thread_count: usize,
    max_file_size: u64,
    show_lines: bool,
    dir_sizes: bool,
    mode: WalkerMode,
}

impl Walker {
    /// Create a new walker with the given options
    pub fn new(root: &Path, filter_opts: FilterOptions, thread_count: usize) -> Result<Self> {
        let root = root.canonicalize()?;
        
        // Load gitignore if requested
        let gitignore = if filter_opts.gitignore {
            load_gitignore(&root)?
        } else {
            None
        };
        
        // Determine optimal walker mode based on requested features
        let mode = Self::determine_mode(&filter_opts, &gitignore);
        
        log::debug!("Walker mode selected: {:?}", mode);
        
        Ok(Self {
            root,
            filter_opts,
            gitignore,
            thread_count: if thread_count == 0 {
                num_cpus::get()
            } else {
                thread_count
            },
            max_file_size: 1_073_741_824, // 1GB default
            show_lines: false,
            dir_sizes: false,
            mode,
        })
    }
    
    /// Set maximum file size for line counting
    pub fn set_max_file_size(&mut self, size: u64) {
        self.max_file_size = size;
    }
    
    /// Enable line counting
    pub fn enable_line_counting(&mut self) {
        self.show_lines = true;
        // Line counting requires at least standard mode
        if matches!(self.mode, WalkerMode::FastPath) {
            self.mode = WalkerMode::Standard;
        }
    }
    
    /// Enable directory size calculation
    pub fn enable_dir_sizes(&mut self) {
        self.dir_sizes = true;
        // Dir sizes require full mode
        self.mode = WalkerMode::Full;
    }
    
    /// Determine the optimal walker mode based on requested features
    fn determine_mode(filter_opts: &FilterOptions, gitignore: &Option<Gitignore>) -> WalkerMode {
        // Check if we need full mode (complex features)
        if filter_opts.search.is_some() ||
           filter_opts.min_size.is_some() ||
           filter_opts.max_size.is_some() ||
           filter_opts.newer_than.is_some() ||
           filter_opts.older_than.is_some() ||
           gitignore.is_some() {
            return WalkerMode::Full;
        }
        
        // Check if we need standard mode (basic filtering)
        if filter_opts.include.is_some() ||
           filter_opts.exclude.is_some() ||
           filter_opts.only_dirs ||
           filter_opts.only_files ||
           filter_opts.sort_by.is_some() ||
           filter_opts.max_dirs.is_some() ||
           filter_opts.max_files.is_some() {
            return WalkerMode::Standard;
        }
        
        // Fast path for simple traversal
        WalkerMode::FastPath
    }
    
    /// Perform the filesystem walk
    pub fn walk(&mut self) -> Result<Vec<TreeEntry>> {
        match self.mode {
            WalkerMode::FastPath => self.walk_fast_path(),
            WalkerMode::Standard => self.walk_standard(),
            WalkerMode::Full => self.walk_full(),
        }
    }
    
    /// Fast path implementation using platform-specific optimizations
    #[cfg(unix)]
    fn walk_fast_path(&mut self) -> Result<Vec<TreeEntry>> {
        // Use syscalls on Unix for maximum speed
        unsafe { self.walk_fast_unix() }
    }
    
    #[cfg(not(unix))]
    fn walk_fast_path(&mut self) -> Result<Vec<TreeEntry>> {
        // Fall back to standard on non-Unix
        self.walk_standard()
    }
    
    /// Fast Unix implementation using syscalls
    #[cfg(unix)]
    unsafe fn walk_fast_unix(&mut self) -> Result<Vec<TreeEntry>> {
        // For fast path, use a simpler recursive approach to avoid tree building complexity
        let root = self.root.clone();
        self.walk_fast_unix_recursive(&root, 0)
    }
    
    /// Recursive helper for fast Unix walker
    #[cfg(unix)]
    unsafe fn walk_fast_unix_recursive(&mut self, path: &Path, depth: usize) -> Result<Vec<TreeEntry>> {
        // Check depth limit
        if let Some(max_depth) = self.filter_opts.max_depth {
            if depth > max_depth {
                return Ok(vec![]);
            }
        }
        
        // Create entry for this path
        let mut entry = self.create_entry_from_path(path, depth)?;
        
        // If it's a directory, recursively process children
        if entry.is_dir && depth < self.filter_opts.max_depth.unwrap_or(usize::MAX) {
            // Convert path to CString
            let path_cstr = path_to_cstring(path)?;
            
            // Open directory
            let dir_handle = libc::opendir(path_cstr.as_ptr());
            if dir_handle.is_null() {
                return Ok(vec![entry]);
            }
            
            // Read directory entries
            loop {
                errno::set_errno(errno::Errno(0));
                let dir_entry = libc::readdir(dir_handle);
                
                if dir_entry.is_null() {
                    let err = errno::errno();
                    if err.0 != 0 {
                        libc::closedir(dir_handle);
                        return Err(Error::IoError(io::Error::last_os_error()));
                    }
                    break;
                }
                
                // Get entry name
                let d_name = (*dir_entry).d_name.as_ptr();
                let name_bytes = CStr::from_ptr(d_name).to_bytes();
                
                // Skip . and ..
                if name_bytes == b"." || name_bytes == b".." {
                    continue;
                }
                
                // Skip hidden files if needed
                if !self.filter_opts.show_hidden && name_bytes.first() == Some(&b'.') {
                    continue;
                }
                
                // Build child path
                let name = OsStr::from_bytes(name_bytes);
                let child_path = path.join(name);
                
                // Recursively process child
                if let Ok(mut child_entries) = self.walk_fast_unix_recursive(&child_path, depth + 1) {
                    if !child_entries.is_empty() {
                        entry.children.push(child_entries.remove(0));
                    }
                }
            }
            
            // Close directory
            libc::closedir(dir_handle);
        }
        
        Ok(vec![entry])
    }
    
    /// Standard implementation with basic filtering
    fn walk_standard(&mut self) -> Result<Vec<TreeEntry>> {
        // Always create root entry, but check if it should be included
        let metadata = fs::symlink_metadata(&self.root)?;
        let mut root_entry = self.create_entry(&self.root, &metadata, 0)?;
        
        // Always process children if root is a directory
        if root_entry.is_dir {
            self.process_directory_children(&mut root_entry)?;
        }
        
        // Only return root if it matches filters or has children
        if self.should_include(&self.root, &metadata) || !root_entry.children.is_empty() {
            Ok(vec![root_entry])
        } else {
            Ok(vec![])
        }
    }
    
    /// Recursively process directory children
    fn process_directory_children(&mut self, parent: &mut TreeEntry) -> Result<()> {
        // Check depth limit
        if let Some(max_depth) = self.filter_opts.max_depth {
            if parent.depth >= max_depth {
                return Ok(());
            }
        }
        
        // Read and process children
        let children_paths = self.read_directory(&parent.path, parent.depth + 1)?;
        
        for child_path in children_paths {
            match self.process_entry(&child_path, parent.depth + 1) {
                Ok(Some(mut child_entry)) => {
                    // Recursively process if it's a directory
                    if child_entry.is_dir {
                        self.process_directory_children(&mut child_entry)?;
                    }
                    parent.children.push(child_entry);
                }
                Ok(None) => {} // Filtered out
                Err(e) => {
                    log::warn!("Error processing {:?}: {}", child_path, e);
                    if !self.filter_opts.gitignore {
                        return Err(e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Full implementation with all features
    fn walk_full(&mut self) -> Result<Vec<TreeEntry>> {
        // Start with standard walk
        let mut entries = self.walk_standard()?;
        
        // Post-process: calculate directory sizes if requested
        if self.dir_sizes {
            self.calculate_dir_sizes(&mut entries)?;
        }
        
        Ok(entries)
    }
    
    /// Create a tree entry from a path
    fn create_entry_from_path(&self, path: &Path, depth: usize) -> Result<TreeEntry> {
        let metadata = fs::symlink_metadata(path)?;
        self.create_entry(path, &metadata, depth)
    }
    
    /// Process a single entry
    fn process_entry(&self, path: &Path, depth: usize) -> Result<Option<TreeEntry>> {
        let metadata = fs::symlink_metadata(path)?;
        
        // Check filters
        if !self.should_include(path, &metadata) {
            return Ok(None);
        }
        
        Ok(Some(self.create_entry(path, &metadata, depth)?))
    }
    
    /// Create entry from path and metadata
    fn create_entry(&self, path: &Path, metadata: &Metadata, depth: usize) -> Result<TreeEntry> {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        let size = if metadata.is_dir() {
            0 // Will be calculated later if requested
        } else {
            metadata.len()
        };
        
        let line_count = if self.show_lines && metadata.is_file() && size <= self.max_file_size {
            count_lines(path, self.max_file_size).unwrap_or(0)
        } else {
            0
        };
        
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let is_symlink = metadata.is_symlink();
        let is_executable = is_executable(metadata);
        
        Ok(TreeEntry {
            name,
            path: path.to_path_buf(),
            size,
            line_count,
            modified,
            is_dir: metadata.is_dir(),
            is_symlink,
            is_executable,
            children: Vec::new(),
            depth,
        })
    }
    
    /// Read directory and return filtered, sorted, limited children
    fn read_directory(&self, path: &Path, depth: usize) -> Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        
        // Read directory entries
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            
            // Apply filters
            if !self.should_include(&path, &metadata) {
                continue;
            }
            
            // Separate dirs and files for limit application
            if metadata.is_dir() {
                dirs.push((path, metadata));
            } else {
                files.push((path, metadata));
            }
        }
        
        // Sort if requested
        if let Some(sort_by) = self.filter_opts.sort_by {
            // Create temporary entries for sorting
            let mut sort_entries: Vec<TreeEntry> = dirs.iter()
                .chain(files.iter())
                .map(|(path, metadata)| {
                    TreeEntry {
                        name: path.file_name().unwrap().to_string_lossy().to_string(),
                        path: path.clone(),
                        size: metadata.len(),
                        line_count: 0,
                        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        is_dir: metadata.is_dir(),
                        is_symlink: metadata.is_symlink(),
                        is_executable: is_executable(metadata),
                        children: Vec::new(),
                        depth,
                    }
                })
                .collect();
            
            sort_entries.sort_by(|a, b| {
                compare_entries(a, b, sort_by, self.filter_opts.reverse_sort)
            });
            
            // Extract sorted paths
            entries = sort_entries.into_iter().map(|e| e.path).collect();
        } else {
            // No sorting, just combine
            entries.extend(dirs.into_iter().map(|(p, _)| p));
            entries.extend(files.into_iter().map(|(p, _)| p));
        }
        
        // Apply limits
        let mut limited = Vec::new();
        let mut dir_count = 0;
        let mut file_count = 0;
        
        for path in entries {
            let is_dir = path.is_dir();
            
            if is_dir {
                if let Some(max_dirs) = self.filter_opts.max_dirs {
                    if dir_count >= max_dirs {
                        continue;
                    }
                }
                dir_count += 1;
            } else {
                if let Some(max_files) = self.filter_opts.max_files {
                    if file_count >= max_files {
                        continue;
                    }
                }
                file_count += 1;
            }
            
            limited.push(path);
        }
        
        Ok(limited)
    }
    
    /// Check if entry should be included based on filters
    fn should_include(&self, path: &Path, metadata: &Metadata) -> bool {
        // Check gitignore
        if let Some(gitignore) = &self.gitignore {
            if gitignore.matched(path, metadata.is_dir()).is_ignore() {
                return false;
            }
        }
        
        // Apply other filters
        self.filter_opts.should_include(path, metadata)
    }
    
    /// Calculate directory sizes recursively
    fn calculate_dir_sizes(&self, entries: &mut [TreeEntry]) -> Result<()> {
        // Use parallel processing for top-level directories
        if self.thread_count > 1 {
            entries.par_iter_mut().try_for_each(|entry| {
                if entry.is_dir {
                    entry.size = calculate_dir_size(&entry.path)?;
                }
                self.calculate_dir_sizes_recursive(&mut entry.children)
            })?;
        } else {
            for entry in entries {
                if entry.is_dir {
                    entry.size = calculate_dir_size(&entry.path)?;
                }
                self.calculate_dir_sizes_recursive(&mut entry.children)?;
            }
        }
        
        Ok(())
    }
    
    /// Recursively calculate directory sizes
    fn calculate_dir_sizes_recursive(&self, entries: &mut [TreeEntry]) -> Result<()> {
        for entry in entries {
            if entry.is_dir {
                entry.size = calculate_dir_size(&entry.path)?;
            }
            self.calculate_dir_sizes_recursive(&mut entry.children)?;
        }
        Ok(())
    }
}

/// Stream walker for direct output without building tree in memory
pub struct StreamWalker<'a> {
    filter_opts: FilterOptions,
    stdout: BufWriter<Box<dyn Write + 'a>>,
    format: FormatterOutputFormat,
    show_size: bool,
    show_lines: bool,
    unicode: bool,
    max_file_size: u64,
    color_enabled: bool,
    file_count: usize,
    dir_count: usize,
}


impl<'a> StreamWalker<'a> {
    /// Create a new stream walker
    pub fn new(
        filter_opts: FilterOptions,
        format: FormatterOutputFormat,
        show_size: bool,
        show_lines: bool,
        unicode: bool,
    ) -> Self {
        let stdout: Box<dyn Write> = Box::new(io::stdout());
        let color_enabled = atty::is(atty::Stream::Stdout);
        Self {
            filter_opts,
            stdout: BufWriter::with_capacity(8192, stdout),
            format,
            show_size,
            show_lines,
            unicode,
            max_file_size: 1_073_741_824, // 1GB default
            color_enabled,
            file_count: 0,
            dir_count: 0,
        }
    }
    
    /// Stream directory tree to stdout
    pub fn stream(&mut self, root: &Path) -> Result<()> {
        match self.format {
            FormatterOutputFormat::Plain => self.stream_plain(root),
            FormatterOutputFormat::Tree => self.stream_tree(root),
            FormatterOutputFormat::Json | FormatterOutputFormat::Csv => {
                // For JSON/CSV, we need to build the full tree first
                Err(Error::general("JSON/CSV output requires full tree building"))
            }
        }
    }
    
    /// Stream plain paths (like find)
    fn stream_plain(&mut self, root: &Path) -> Result<()> {
        // Just output full paths, one per line
        let walker = Walker::new(root, self.filter_opts.clone(), 1)?;
        self.walk_and_print_plain(&walker, root, 0)?;
        self.stdout.flush()?;
        Ok(())
    }
    
    
    /// Stream tree format
    fn stream_tree(&mut self, root: &Path) -> Result<()> {
        let walker = Walker::new(root, self.filter_opts.clone(), 1)?;
        self.walk_and_print_tree(&walker, root, 0, &mut Vec::new())?;
        self.stdout.flush()?;
        
        // Print summary line like tree command
        println!();
        println!("{} {}, {} {}",
            self.dir_count,
            if self.dir_count == 1 { "directory" } else { "directories" },
            self.file_count,
            if self.file_count == 1 { "file" } else { "files" }
        );
        
        Ok(())
    }
    
    /// Walk and print plain paths
    fn walk_and_print_plain(&mut self, walker: &Walker, path: &Path, depth: usize) -> Result<()> {
        // Check depth
        if let Some(max_depth) = walker.filter_opts.max_depth {
            if depth > max_depth {
                return Ok(());
            }
        }
        
        // Print path
        writeln!(self.stdout, "{}", path.display())?;
        
        // Recurse if directory
        if path.is_dir() && depth < walker.filter_opts.max_depth.unwrap_or(usize::MAX) {
            let children = walker.read_directory(path, depth + 1)?;
            for child in children {
                self.walk_and_print_plain(walker, &child, depth + 1)?;
            }
        }
        
        Ok(())
    }
    
    
    /// Walk and print tree
    fn walk_and_print_tree(
        &mut self,
        walker: &Walker,
        path: &Path,
        depth: usize,
        prefix: &mut Vec<bool>,
    ) -> Result<()> {
        // Check depth
        if let Some(max_depth) = walker.filter_opts.max_depth {
            if depth > max_depth {
                return Ok(());
            }
        }
        
        // Print tree line
        if depth > 0 {
            // Print prefix
            for (i, &is_last) in prefix.iter().enumerate() {
                if i == prefix.len() - 1 {
                    write!(self.stdout, "{}", if is_last {
                        if self.unicode { "└── " } else { "`-- " }
                    } else {
                        if self.unicode { "├── " } else { "|-- " }
                    })?;
                } else {
                    write!(self.stdout, "{}", if is_last { "    " } else {
                        if self.unicode { "│   " } else { "|   " }
                    })?;
                }
            }
        }
        
        // Get metadata for the path
        let metadata = fs::symlink_metadata(path).ok();
        let is_dir = metadata.as_ref().map_or(false, |m| m.is_dir());
        let is_symlink = metadata.as_ref().map_or(false, |m| m.is_symlink());
        let is_executable = metadata.as_ref().map_or(false, |m| is_executable(m));
        let size = metadata.as_ref().map_or(0, |m| m.len());
        
        // Update counts
        if is_dir {
            self.dir_count += 1;
        } else {
            self.file_count += 1;
        }
        
        // Get name
        let name = if depth == 0 {
            path.to_string_lossy().to_string()
        } else {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string())
        };
        
        // Apply color based on file type
        let colored_name = if self.color_enabled {
            if is_dir {
                name.blue().bold().to_string()
            } else if is_symlink {
                name.cyan().to_string()
            } else if is_executable {
                name.green().to_string()
            } else {
                name
            }
        } else {
            name
        };
        
        // Build the output line
        let mut output = colored_name;
        
        // Add size and line count if requested
        if metadata.is_some() {
            let mut details = Vec::new();
            
            if self.show_size && !is_dir {
                details.push(crate::formatter::format_size(size));
            }
            
            if self.show_lines && !is_dir && size <= self.max_file_size {
                if let Ok(lines) = count_lines(path, self.max_file_size) {
                    if lines > 0 {
                        details.push(format!("{} lines", lines));
                    }
                }
            }
            
            if !details.is_empty() {
                let details_str = format!(" ({})", details.join(", "));
                output.push_str(&if self.color_enabled {
                    details_str.dimmed().to_string()
                } else {
                    details_str
                });
            }
        }
        
        writeln!(self.stdout, "{}", output)?;
        
        // Recurse if directory
        if path.is_dir() && depth < walker.filter_opts.max_depth.unwrap_or(usize::MAX) {
            let children = walker.read_directory(path, depth + 1)?;
            let child_count = children.len();
            
            for (i, child) in children.into_iter().enumerate() {
                let is_last = i == child_count - 1;
                prefix.push(is_last);
                self.walk_and_print_tree(walker, &child, depth + 1, prefix)?;
                prefix.pop();
            }
        }
        
        Ok(())
    }
}

/// Load gitignore patterns from directory tree
fn load_gitignore(root: &Path) -> Result<Option<Gitignore>> {
    let mut builder = GitignoreBuilder::new(root);
    
    // Add .gitignore from root
    let gitignore_path = root.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(&gitignore_path);
    }
    
    // Build gitignore
    match builder.build() {
        Ok(gitignore) => Ok(Some(gitignore)),
        Err(e) => {
            log::warn!("Failed to load gitignore: {}", e);
            Ok(None)
        }
    }
}

/// Convert Path to CString for Unix syscalls
#[cfg(unix)]
#[inline(always)]
fn path_to_cstring(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| Error::path("Invalid path"))
}

/// Check if file is executable
#[cfg(unix)]
fn is_executable(metadata: &Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    
    #[test]
    fn test_walker_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create test structure
        fs::create_dir(root.join("subdir")).unwrap();
        File::create(root.join("file1.txt")).unwrap();
        File::create(root.join("subdir/file2.txt")).unwrap();
        
        // Walk directory
        let filter_opts = FilterOptions {
            show_hidden: true,
            ..Default::default()
        };
        let mut walker = Walker::new(root, filter_opts, 1).unwrap();
        let entries = walker.walk().unwrap();
        
        // Check results
        assert_eq!(entries.len(), 1); // Root
        assert_eq!(entries[0].children.len(), 2); // subdir and file1.txt
    }
    
    #[test]
    fn test_walker_depth_limit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        // Create nested structure
        fs::create_dir_all(root.join("a/b/c/d")).unwrap();
        
        // Walk with depth limit
        let filter_opts = FilterOptions {
            max_depth: Some(2),
            show_hidden: true,
            ..Default::default()
        };
        let mut walker = Walker::new(root, filter_opts, 1).unwrap();
        let entries = walker.walk().unwrap();
        
        // Verify depth limit
        fn check_max_depth(entry: &TreeEntry, max: usize) -> bool {
            if entry.depth > max {
                return false;
            }
            entry.children.iter().all(|c| check_max_depth(c, max))
        }
        
        assert!(entries.iter().all(|e| check_max_depth(e, 2)));
    }
}