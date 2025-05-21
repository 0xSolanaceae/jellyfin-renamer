#![windows_subsystem = "windows"]

use std::env;
use std::path::Path;
use std::fs;

mod popup;

fn main() {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    
    // Check if a file path was provided
    if args.len() < 2 {
        return;
    }
    
    let file_path = &args[1];
    
    // Show confirmation dialog
    if popup::show_rename_dialog(file_path) {
        rename_to_helloworld(file_path);
    }
}

fn rename_to_helloworld(file_path: &str) {
    let path = Path::new(file_path);
    
    // Make sure the file exists
    if !path.exists() {
        return;
    }
    
    // Get parent directory
    let parent = match path.parent() {
        Some(p) => p,
        None => {
            return;
        }
    };
    
    // Get the file extension (if any)
    let extension = match path.extension() {
        Some(ext) => format!(".{}", ext.to_string_lossy()),
        None => String::new(),
    };
    
    // Create the new name with original extension
    let new_name = format!("helloworld{}", extension);
    
    // Create the new path
    let new_path = parent.join(new_name);
    
    // Rename the file
    match fs::rename(path, &new_path) {
        Ok(_) => {},
        Err(_) => {},
    }
}