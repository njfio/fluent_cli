# Directory Listing in Rust

This program lists all files and directories in the current working directory using Rust's standard library.

```rust
use std::fs;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    // Get the current directory
    let current_dir = std::env::current_dir()?;
    println!("Contents of directory: {}", current_dir.display());
    
    // Read the directory entries
    let entries = fs::read_dir(current_dir)?;
    
    // Print header
    println!("\n{:<40} {:<10} {:<12}", "Name", "Type", "Size (bytes)");
    println!("{:-<40} {:-<10} {:-<12}", "", "", "");
    
    // Process each entry
    for entry_result in entries {
        let entry = entry_result?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        
        // Get file name
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("[Invalid UTF-8 filename]");
        
        // Determine if it's a file or directory
        let file_type = if metadata.is_dir() {
            "Directory"
        } else if metadata.is_file() {
            "File"
        } else if metadata.is_symlink() {
            "Symlink"
        } else {
            "Other"
        };
        
        // Get file size
        let size = metadata.len();
        
        println!("{:<40} {:<10} {:<12}", name, file_type, size);
    }
    
    Ok(())
}
```

## How to Use

1. Save this code to a file named `list_directory.rs`
2. Compile it with `rustc list_directory.rs`
3. Run the resulting executable: `./list_directory` (or `list_directory.exe` on Windows)

## What This Code Does

This program:

1. Gets the current working directory using `std::env::current_dir()`
2. Lists all entries in that directory using `fs::read_dir()`
3. For each entry, displays:
   - The name of the file or directory
   - Whether it's a file, directory, symlink, or other type
   - The size in bytes (for files)

The output is formatted in a table for better readability.

## Error Handling

The code uses Rust's `Result` type for proper error handling. Potential errors that could occur include:
- Permission issues when accessing directories
- I/O errors when reading directory entries
- Invalid UTF-8 in filenames

These errors are propagated up to the `main` function, which returns a `Result` type.