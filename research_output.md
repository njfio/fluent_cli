# Directory Listing Program in Rust

I'll create a comprehensive Rust program that lists files in the current directory with detailed information.

```rust
use std::fs;
use std::io;
use std::path::Path;
use std::time::{UNIX_EPOCH, SystemTime};
use std::os::unix::fs::PermissionsExt;

fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
    }
}

fn format_permissions(mode: u32) -> String {
    let user = [(mode & 0o400) >> 8, (mode & 0o200) >> 7, (mode & 0o100) >> 6];
    let group = [(mode & 0o40) >> 5, (mode & 0o20) >> 4, (mode & 0o10) >> 3];
    let other = [(mode & 0o4) >> 2, (mode & 0o2) >> 1, mode & 0o1];
    
    let mut result = String::new();
    
    for bits in [user, group, other] {
        result.push(if bits[0] != 0 { 'r' } else { '-' });
        result.push(if bits[1] != 0 { 'w' } else { '-' });
        result.push(if bits[2] != 0 { 'x' } else { '-' });
    }
    
    result
}

fn format_time(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    
    // Simple formatting - in a real app you might want to use chrono crate
    let seconds = secs % 60;
    let minutes = (secs / 60) % 60;
    let hours = (secs / 3600) % 24;
    let days = secs / 86400;
    
    format!("{} days, {:02}:{:02}:{:02}", days, hours, minutes, seconds)
}

fn list_directory_contents(path: &Path) -> io::Result<()> {
    let mut entries = Vec::new();
    let mut total_size = 0;
    let mut file_count = 0;
    let mut dir_count = 0;
    
    println!("Contents of directory: {}", path.display());
    println!("{:-<80}", "");
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_type = if metadata.is_dir() {
            dir_count += 1;
            "Directory"
        } else if metadata.is_file() {
            file_count += 1;
            total_size += metadata.len();
            "File"
        } else if metadata.is_symlink() {
            "Symlink"
        } else {
            "Other"
        };
        
        let size = format_file_size(metadata.len());
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let modified_str = format_time(modified);
        
        #[cfg(unix)]
        let permissions = format_permissions(metadata.permissions().mode());
        #[cfg(not(unix))]
        let permissions = if metadata.permissions().readonly() {
            "read-only"
        } else {
            "read-write"
        };
        
        entries.push((
            entry.file_name().to_string_lossy().to_string(),
            file_type.to_string(),
            size,
            modified_str,
            permissions,
        ));
    }
    
    // Sort entries by name
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    
    // Print header
    println!("{:<30} {:<10} {:<15} {:<25} {:<10}", 
             "Name", "Type", "Size", "Modified", "Permissions");
    println!("{:-<80}", "");
    
    // Print entries
    for (name, file_type, size, modified, permissions) in entries {
        println!("{:<30} {:<10} {:<15} {:<25} {:<10}", 
                 name, file_type, size, modified, permissions);
    }
    
    println!("{:-<80}", "");
    println!("Summary: {} files, {} directories, Total size: {}", 
             file_count, dir_count, format_file_size(total_size));
    
    Ok(())
}

fn main() -> io::Result<()> {
    let current_dir = std::env::current_dir()?;
    list_directory_contents(&current_dir)?;
    Ok(())
}
```

## How to Use This Program

1. Save the code above to a file named `list_files.rs`
2. Compile it with: `rustc list_files.rs`
3. Run the resulting executable: `./list_files`

## Features

- Lists all files and directories in the current working directory
- Shows file type (file, directory, symlink)
- Displays file size in human-readable format (bytes, KB, MB, GB)
- Shows file modification time
- Displays file permissions in Unix-style format on Unix systems
- Provides a summary with total file count, directory count, and total size
- Sorts entries alphabetically by name
- Handles errors gracefully

## Sample Output

When run, the program will produce output similar to:

```
Contents of directory: /path/to/current/directory
--------------------------------------------------------------------------------
Name                           Type       Size            Modified                  Permissions
--------------------------------------------------------------------------------
.gitignore                     File       124 bytes       0 days, 12:34:56         rw-r--r--
Cargo.toml                     File       342 bytes       1 days, 08:15:30         rw-r--r--
README.md                      File       1.25 KB         0 days, 14:22:18         rw-r--r--
src                            Directory  0 bytes         2 days, 09:45:12         rwxr-xr-x
target                         Directory  0 bytes         0 days, 10:30:45         rwxr-xr-x
--------------------------------------------------------------------------------
Summary: 3 files, 2 directories, Total size: 1.72 KB
```

This program provides a comprehensive view of the current directory's contents with detailed information about each file and directory.