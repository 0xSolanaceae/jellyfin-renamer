use std::env;
use std::path::Path;

mod rename_engine;
mod instance_coordinator;
mod tui;

use instance_coordinator::InstanceCoordinator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {    let args: Vec<String> = env::args().collect();
    
    let selected_files = if args.len() >= 2 {
        let coordinator = InstanceCoordinator::new();
        let collected_files = coordinator.collect_files_from_instances(&args[1]);
        
        if collected_files.is_none() {
            return Ok(());
        }
        
        let collected_files = collected_files.unwrap();
        
        if collected_files.len() > 1 {
            collected_files
        } else {
            let path = Path::new(&args[1]);
            if path.is_file() {
                vec![args[1].clone()]
            } else {
                vec![]
            }
        }
    } else {
        vec![]
    };    let directory_arg = if args.len() >= 2 {
        let path = Path::new(&args[1]);
        if path.is_dir() {
            Some(args[1].clone())
        } else {
            None
        }
    } else {
        None
    };

    tui::run_tui(directory_arg, selected_files).await?;

    Ok(())
}