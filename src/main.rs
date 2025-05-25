use std::env;
use std::io;

mod rename_engine;
mod instance_coordinator;

use rename_engine::rename_file;
use instance_coordinator::InstanceCoordinator;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Jellyfin Rename Tool");
        println!("===================");
        println!("Usage: {} <file_path> [additional_files...]", args[0]);
        println!("\nNo files provided to rename.");
        pause_and_exit();
        return;
    }

    // Try to collect files from multiple instances
    let coordinator = InstanceCoordinator::new();
    let collected_files = coordinator.collect_files_from_instances(&args[1]);
    
    // If we're not the first instance, exit silently
    if collected_files.is_none() {
        return;
    }
    
    let collected_files = collected_files.unwrap();
    
    println!("Jellyfin Rename Tool");
    println!("===================");
    
    // Log the arguments for debugging
    println!("Arguments received: {:?}", args);
    println!("Collected files: {:?}", collected_files);
    
    let file_paths = if collected_files.len() > 1 {
        collected_files
    } else {
        args[1..].to_vec()
    };
    
    println!("Processing {} file(s)...\n", file_paths.len());
    
    let mut success_count = 0;
    let mut total_count = 0;    // Process each file
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

fn pause_and_exit() {
    println!("\nPress Enter to close this window...");
    let _ = io::stdin().read_line(&mut String::new());
}