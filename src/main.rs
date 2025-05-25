use std::env;

mod popup;
mod rename_engine;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        return;
    }
    
    let file_paths: Vec<String> = args[1..].to_vec();
    
    // Process each file
    for file_path in &file_paths {
        // Show dialog and perform rename if confirmed
        popup::show_rename_dialog(file_path);
        // The renaming is now handled inside the popup module's callback
    }
}