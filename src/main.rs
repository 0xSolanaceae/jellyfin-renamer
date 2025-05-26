use std::env;
use std::io;
use std::path::Path;

mod rename_engine;
mod instance_coordinator;
mod tui;

use instance_coordinator::InstanceCoordinator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    // Collect files from arguments
    let selected_files = if args.len() >= 2 {
        // Try to collect files from multiple instances
        let coordinator = InstanceCoordinator::new();
        let collected_files = coordinator.collect_files_from_instances(&args[1]);
        
        // If we're not the first instance, exit silently
        if collected_files.is_none() {
            return Ok(());
        }
        
        let collected_files = collected_files.unwrap();
        
        if collected_files.len() > 1 {
            // Multiple files were collected
            collected_files
        } else {
            // Single file/directory provided
            let path = Path::new(&args[1]);
            if path.is_file() {
                vec![args[1].clone()]
            } else {
                // If it's a directory, we'll scan it in the TUI
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Get the directory from the first file or use the argument if it's a directory
    let directory = if !selected_files.is_empty() {
        if let Some(first_file) = selected_files.first() {
            Path::new(first_file)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        }
    } else if args.len() >= 2 {
        let path = Path::new(&args[1]);
        if path.is_dir() {
            Some(args[1].clone())
        } else {
            None
        }
    } else {
        None
    };

    // Launch the TUI with pre-selected files
    tui::run_tui(directory, selected_files).await?;

    Ok(())
}

fn pause_and_exit() {
    println!("\nPress Enter to close this window...");
    let _ = io::stdin().read_line(&mut String::new());
}