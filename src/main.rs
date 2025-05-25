use std::env;
use std::path::Path;
use std::io;

mod rename_engine;

use rename_engine::{RenameOperation, RenameResult};

fn main() {
    println!("Jellyfin Rename Tool");
    println!("===================");
    
    let args: Vec<String> = env::args().collect();
    
    // Log the arguments for debugging
    println!("Arguments received: {:?}", args);
    
    if args.len() < 2 {
        println!("Usage: {} <file_path> [additional_files...]", args[0]);
        println!("\nNo files provided to rename.");
        pause_and_exit();
        return;
    }
    
    let file_paths: Vec<String> = args[1..].to_vec();
    println!("Processing {} file(s)...\n", file_paths.len());
    
    let mut success_count = 0;
    let mut total_count = 0;
    
    // Process each file
    for file_path in &file_paths {
        total_count += 1;
        println!("--- Processing file {} of {} ---", total_count, file_paths.len());
        if rename_file(file_path) {
            success_count += 1;
        }
        println!();
    }
    
    // Summary
    println!("===================");
    println!("Summary: {} of {} files processed successfully", success_count, total_count);
    
    if success_count == total_count && total_count > 0 {
        println!("✓ All files renamed successfully!");
    } else if success_count > 0 {
        println!("⚠ Some files were renamed, but there were errors with others.");
    } else {
        println!("✗ No files were renamed.");
    }
    
    pause_and_exit();
}

fn rename_file(file_path: &str) -> bool {
    println!("File path: {}", file_path);
    
    let path = Path::new(file_path);
    
    if !path.exists() {
        println!("✗ Error: File '{}' not found!", file_path);
        return false;
    }

    if !path.is_file() {
        println!("✗ Error: '{}' is not a file!", file_path);
        return false;
    }

    let mut rename_op = RenameOperation::new(file_path);
    
    // Get the original filename without extension
    let original_name = rename_op.get_name_without_extension();
    println!("Original name: {}", original_name);
    
    // Generate new name by processing the original name
    let new_name = process_filename(original_name);
    println!("Processed name: {}", new_name);
    
    // If the name doesn't change, skip renaming
    if new_name == original_name {
        println!("ℹ No changes needed for: {}", rename_op.get_original_name());
        return true; // Consider this a success since no action was needed
    }
    
    // Update the rename operation with the new name
    rename_op.update_new_name(new_name);
    
    println!("Renaming: {} -> {}", rename_op.get_original_name(), rename_op.get_new_name());
    
    // Execute the rename
    match rename_op.execute() {
        RenameResult::Success(_) => {
            println!("✓ Successfully renamed to: {}", rename_op.get_new_name());
            true
        }
        RenameResult::AlreadyExists => {
            println!("✗ Error: Target file '{}' already exists!", rename_op.get_new_name());
            false
        }
        RenameResult::NoPermission => {
            println!("✗ Error: No permission to rename '{}'!", file_path);
            false
        }
        RenameResult::SourceNotFound => {
            println!("✗ Error: Source file '{}' not found!", file_path);
            false
        }
        RenameResult::OtherError(msg) => {
            println!("✗ Error renaming '{}': {}", file_path, msg);
            false
        }
    }
}

fn process_filename(filename: &str) -> String {
    println!("Debug: Starting with filename: '{}'", filename);
    
    let mut processed = filename.to_string();
    
    // Remove common unwanted patterns (case insensitive)
    let patterns_to_remove = [
        // Torrent sites and groups
        "www.yts.mx", "YTS.MX", "yts.mx", "[YTS.MX]", "(YTS.MX)",
        "YIFY", "yify", "[YIFY]", "(YIFY)",
        "RARBG", "rarbg", "[RARBG]", "(RARBG)",
        "1337x", "[1337x]", "(1337x)",
        "TGx", "[TGx]", "(TGx)",
        
        // Video quality indicators
        "1080p", "720p", "480p", "2160p", "4K", "HD", "HDTV",
        "BluRay", "Blu-Ray", "BrRip", "BDRip", "DVDRip", "WEBRip", "WEB-DL",
        "CAMRip", "TS", "TC", "SCREENER", "R5",
        
        // Video codecs
        "x264", "x265", "H.264", "H.265", "HEVC", "AVC", "XviD", "DivX",
        "VP9", "AV1",
        
        // Audio codecs
        "AAC", "AC3", "DTS", "MP3", "FLAC", "Atmos", "DTS-HD",
        
        // Release info
        "EXTENDED", "UNRATED", "DIRECTORS.CUT", "REMASTERED",
        "LIMITED", "INTERNAL", "PROPER", "REPACK",
    ];
    
    // Case insensitive pattern removal
    for pattern in &patterns_to_remove {
        let pattern_lower = pattern.to_lowercase();
        let original_len = processed.len();
        
        // Keep replacing until no more matches found
        loop {
            let processed_lower = processed.to_lowercase();
            if let Some(pos) = processed_lower.find(&pattern_lower) {
                processed = format!("{}{}", &processed[..pos], &processed[pos + pattern.len()..]);
            } else {
                break;
            }
        }
        
        if processed.len() != original_len {
            println!("Debug: Removed '{}', now: '{}'", pattern, processed);
        }
    }
    
    // Remove brackets and their contents if they seem to be release info
    let bracket_patterns = [
        (r"\[.*?\]", "square brackets"),
        (r"\(.*?\)", "parentheses"),
    ];
    
    for (pattern, desc) in &bracket_patterns {
        let original = processed.clone();
        // Simple bracket removal - replace with space
        processed = processed.chars().collect::<Vec<char>>()
            .iter()
            .fold((String::new(), 0, false), |(mut result, depth, in_brackets), &ch| {
                match ch {
                    '[' | '(' => {
                        if depth == 0 { result.push(' '); }
                        (result, depth + 1, true)
                    },
                    ']' | ')' => {
                        let new_depth = if depth > 0 { depth - 1 } else { 0 };
                        if new_depth == 0 { result.push(' '); }
                        (result, new_depth, new_depth > 0)
                    },
                    _ => {
                        if !in_brackets {
                            result.push(ch);
                        }
                        (result, depth, in_brackets)
                    }
                }
            }).0;
            
        if processed != original {
            println!("Debug: Removed {}, now: '{}'", desc, processed);
        }
    }
    
    // Replace common separators with spaces
    let separators = [".", "_", "-", "+"];
    for sep in &separators {
        if processed.contains(sep) {
            processed = processed.replace(sep, " ");
            println!("Debug: Replaced '{}' with spaces, now: '{}'", sep, processed);
        }
    }
    
    // Clean up multiple spaces and trim
    while processed.contains("  ") {
        processed = processed.replace("  ", " ");
    }
    
    processed = processed.trim().to_string();
    
    // Remove leading/trailing dots, spaces, and other punctuation
    processed = processed.trim_matches(&['.', ' ', '-', '_', '(', ')', '[', ']', ','] as &[char]).to_string();
    
    // Capitalize first letter of each word for better presentation
    let words: Vec<String> = processed
        .split_whitespace()
        .map(|word| {
            let mut chars: Vec<char> = word.chars().collect();
            if !chars.is_empty() {
                chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
            }
            chars.into_iter().collect()
        })
        .collect();
    
    processed = words.join(" ");
    
    println!("Debug: Final result: '{}'", processed);
    
    processed
}

fn pause_and_exit() {
    println!("\nPress Enter to close this window...");
    let _ = io::stdin().read_line(&mut String::new());
}