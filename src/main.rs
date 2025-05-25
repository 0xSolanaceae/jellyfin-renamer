use std::env;
use std::path::Path;

mod rename_engine;

use rename_engine::{RenameOperation, RenameResult};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path> [additional_files...]", args[0]);
        return;
    }
    
    let file_paths: Vec<String> = args[1..].to_vec();
    
    // Process each file
    for file_path in &file_paths {
        rename_file(file_path);
    }
}

fn rename_file(file_path: &str) {
    let path = Path::new(file_path);
    
    if !path.exists() {
        eprintln!("Error: File '{}' not found!", file_path);
        return;
    }

    if !path.is_file() {
        eprintln!("Error: '{}' is not a file!", file_path);
        return;
    }

    let mut rename_op = RenameOperation::new(file_path);
    
    // Get the original filename without extension
    let original_name = rename_op.get_name_without_extension();
    
    // Generate new name by processing the original name
    let new_name = process_filename(original_name);
    
    // If the name doesn't change, skip renaming
    if new_name == original_name {
        println!("No changes needed for: {}", rename_op.get_original_name());
        return;
    }
    
    // Update the rename operation with the new name
    rename_op.update_new_name(new_name);
    
    println!("Renaming: {} -> {}", rename_op.get_original_name(), rename_op.get_new_name());
    
    // Execute the rename
    match rename_op.execute() {
        RenameResult::Success(_) => {
            println!("✓ Successfully renamed to: {}", rename_op.get_new_name());
        }
        RenameResult::AlreadyExists => {
            eprintln!("✗ Error: Target file '{}' already exists!", rename_op.get_new_name());
        }
        RenameResult::NoPermission => {
            eprintln!("✗ Error: No permission to rename '{}'!", file_path);
        }
        RenameResult::SourceNotFound => {
            eprintln!("✗ Error: Source file '{}' not found!", file_path);
        }
        RenameResult::OtherError(msg) => {
            eprintln!("✗ Error renaming '{}': {}", file_path, msg);
        }
    }
}

fn process_filename(filename: &str) -> String {
    // Add your filename processing logic here
    // For now, this is a simple example that removes common prefixes/suffixes
    // and cleans up the filename for Jellyfin
    
    let mut processed = filename.to_string();
    
    // Remove common unwanted patterns (customize as needed)
    let patterns_to_remove = [
        "www.yts.mx",
        "YTS.MX",
        "YIFY",
        "[YTS.MX]",
        "1080p",
        "720p",
        "BluRay",
        "WEBRip",
        "x264",
        "x265",
        "HEVC",
    ];
    
    for pattern in &patterns_to_remove {
        processed = processed.replace(pattern, "");
    }
    
    // Clean up multiple spaces and trim
    while processed.contains("  ") {
        processed = processed.replace("  ", " ");
    }
    
    processed = processed.trim().to_string();
    
    // Remove leading/trailing dots and spaces
    processed = processed.trim_matches(&['.', ' '] as &[char]).to_string();
    
    processed
}