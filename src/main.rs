use std::env;
use std::io;

mod rename_engine;
mod instance_coordinator;
mod tui;

use instance_coordinator::InstanceCoordinator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Jellyfin Rename Tool");
        println!("===================");
        println!("Usage: {} <file_path> [additional_files...]", args[0]);
        println!("\nNo files provided to rename.");
        pause_and_exit();
        return Ok(());
    }

    // Try to collect files from multiple instances
    let coordinator = InstanceCoordinator::new();
    let collected_files = coordinator.collect_files_from_instances(&args[1]);
    
    // If we're not the first instance, exit silently
    if collected_files.is_none() {
        return Ok(());
    }
    
    let collected_files = collected_files.unwrap();
    
    let file_paths = if collected_files.len() > 1 {
        collected_files
    } else {
        args[1..].to_vec()
    };

    // Launch the beautiful TUI
    tui::run_tui(file_paths).await?;

    Ok(())
}

fn pause_and_exit() {
    println!("\nPress Enter to close this window...");
    let _ = io::stdin().read_line(&mut String::new());
}