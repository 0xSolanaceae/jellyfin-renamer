#![windows_subsystem = "windows"]

use std::env;
use std::path::Path;
use std::fs;

mod popup;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        return;
    }
    
    let file_paths: Vec<String> = args[1..].to_vec();
    
    // Show dialog for all files, not just the first one
    let mut should_rename = true;
    for file_path in &file_paths {
        if !popup::show_rename_dialog(file_path) {
            should_rename = false;
            break;
        }
    }
    if should_rename {
        rename_to_helloworld(&file_paths);
    }
}

fn rename_to_helloworld(file_paths: &[String]) {
    // Use a counter to make filenames unique when there are multiple files
    let mut counter = 0;
    
    for file_path in file_paths {
        let path = Path::new(file_path);
        
        if !path.exists() {
            continue;
        }
        
        let parent = match path.parent() {
            Some(p) => p,
            None => {
                continue;
            }
        };
        
        let extension = match path.extension() {
            Some(ext) => format!(".{}", ext.to_string_lossy()),
            None => String::new(),
        };
        
        // Create unique filename with counter if there are multiple files
        let new_name = if file_paths.len() > 1 {
            counter += 1;
            format!("helloworld_{}{}", counter, extension)
        } else {
            format!("helloworld{}", extension)
        };
        
        let new_path = parent.join(new_name);
        
        match fs::rename(path, &new_path) {
            Ok(_) => {},
            Err(_) => {},
        }
    }
}