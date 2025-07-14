//! Configuration file handling
//!
//! This module manages loading and parsing configuration from ~/.maram.toml

use crate::{Error, Result};
use crate::filters::SortBy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Display options
    #[serde(default)]
    pub display: DisplayConfig,
    
    /// Filter options
    #[serde(default)]
    pub filters: FilterConfig,
    
    /// Performance options
    #[serde(default)]
    pub performance: PerformanceConfig,
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Use Unicode characters by default
    #[serde(default)]
    pub unicode: bool,
    
    /// Show file sizes by default
    #[serde(default = "default_true")]
    pub show_size: bool,
    
    /// Show line counts by default
    #[serde(default)]
    pub show_lines: bool,
    
    /// Show directory sizes by default
    #[serde(default)]
    pub dir_sizes: bool,
    
    /// Show total size by default
    #[serde(default = "default_true")]
    pub total_size: bool,
}

/// Filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Show hidden files by default
    #[serde(default)]
    pub show_hidden: bool,
    
    /// Respect gitignore by default
    #[serde(default)]
    pub gitignore: bool,
    
    /// Default max depth
    #[serde(default)]
    pub max_depth: Option<usize>,
    
    /// Default max directories per level
    #[serde(default)]
    pub max_dirs: Option<usize>,
    
    /// Default max files per level
    #[serde(default)]
    pub max_files: Option<usize>,
    
    /// Default sort field
    #[serde(default)]
    pub sort_by: Option<SortBy>,
    
    /// Reverse sort by default
    #[serde(default)]
    pub reverse_sort: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of threads (0 = auto)
    #[serde(default)]
    pub threads: usize,
    
    /// Maximum file size for line counting
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            unicode: true,
            show_size: true,
            show_lines: false,
            dir_sizes: false,
            total_size: true,
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            gitignore: false,
            max_depth: None,
            max_dirs: None,
            max_files: None,
            sort_by: None,
            reverse_sort: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            threads: 0,
            max_file_size: 1_073_741_824, // 1GB
        }
    }
}

impl Config {
    /// Load configuration from ~/.maram.toml
    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        
        if !config_path.exists() {
            log::debug!("No config file found at {:?}, using defaults", config_path);
            return Ok(Self::default());
        }
        
        log::debug!("Loading config from {:?}", config_path);
        let content = fs::read_to_string(&config_path)
            .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;
        
        let config: Config = toml::from_str(&content)?;
        
        Ok(config)
    }
    
    /// Save configuration to ~/.maram.toml
    pub fn save(&self) -> Result<()> {
        let config_path = get_config_path()?;
        
        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::config(format!("Failed to create config directory: {}", e)))?;
        }
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::config(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(&config_path, content)
            .map_err(|e| Error::config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
}

/// Get the path to the config file
fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| Error::config("Could not determine home directory"))?;
    
    Ok(home.join(".maram.toml"))
}

/// Default value helpers for serde
fn default_true() -> bool {
    true
}

fn default_max_file_size() -> u64 {
    1_073_741_824 // 1GB
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.display.show_size);
        assert!(!config.display.unicode);
        assert_eq!(config.performance.max_file_size, 1_073_741_824);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.display.show_size, parsed.display.show_size);
        assert_eq!(config.filters.gitignore, parsed.filters.gitignore);
    }
}