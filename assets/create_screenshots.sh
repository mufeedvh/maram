#!/bin/bash

# Create assets directory if it doesn't exist
mkdir -p assets

# Remove old screenshots
rm -f assets/maram-*.png

# Create demo directory structure
mkdir -p demo/{src,docs,tests}

# Create demo files
cat > demo/src/main.rs << 'EOF'
fn main() {
    println!("Hello, maram!");
    // Main application entry point
    run_app();
}

fn run_app() {
    // Application logic here
}
EOF

cat > demo/src/lib.rs << 'EOF'
pub mod utils;
pub mod core;

pub fn init() {
    println!("Initializing maram");
}
EOF

cat > demo/src/utils.rs << 'EOF'
use std::fs;
use std::path::Path;

pub fn read_dir(path: &Path) -> std::io::Result<Vec<String>> {
    let entries = fs::read_dir(path)?;
    let mut names = Vec::new();
    
    for entry in entries {
        let entry = entry?;
        names.push(entry.file_name().to_string_lossy().to_string());
    }
    
    Ok(names)
}
EOF

cat > demo/src/core.rs << 'EOF'
pub struct Tree {
    pub name: String,
    pub children: Vec<Tree>,
}

impl Tree {
    pub fn new(name: String) -> Self {
        Tree {
            name,
            children: Vec::new(),
        }
    }
}
EOF

echo '# Demo Project

A demo for maram screenshots.' > demo/README.md

echo '{
  "name": "demo",
  "version": "1.0.0"
}' > demo/package.json

echo 'fn test_tree() {
    assert!(true);
}' > demo/tests/test.rs

echo '# API Documentation

API reference for demo project.' > demo/docs/api.md

dd if=/dev/zero of=demo/docs/manual.pdf bs=512K count=1 2>/dev/null

echo "Creating screenshots with full content..."

# 1. Basic tree view with full output
termshot -C 120 --show-cmd --filename assets/maram-full-tree.png -- maram demo --show-size

# 2. With file sizes and line counts
termshot -C 120 --show-cmd --filename assets/maram-showcase-full.png -- maram demo --show-size --show-lines

# 3. Size distribution chart
termshot -C 120 --show-cmd --filename assets/maram-distribution-chart.png -- maram demo --dist=ext --format=chart

# 4. Unicode tree with colors
termshot -C 120 --show-cmd --filename assets/maram-unicode-tree.png -- maram demo -u --show-size

# 5. Filtering example
termshot -C 120 --show-cmd --filename assets/maram-filtering-example.png -- maram src --include='\.rs$' --show-size --sort=size --reverse

# 6. Project overview with limits
termshot -C 120 --show-cmd --filename assets/maram-project-overview.png -- maram . --show-size --show-lines --depth=2 --max-files=5 --max-dirs=3

# Clean up demo directory
rm -rf demo

echo "Screenshots created successfully!"
echo "Files created:"
ls -la assets/maram-*.png