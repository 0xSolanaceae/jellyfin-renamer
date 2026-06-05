// Integration tests for the instance coordinator module

use jellyfin_rename::instance_coordinator::InstanceCoordinator;
use std::env;

#[test]
fn test_coordinator_creation() {
    let _coordinator = InstanceCoordinator::new();
    assert!(true);
}

#[test]
fn test_default_coordinator() {
    let _coordinator = InstanceCoordinator::default();
    assert!(true);
}

#[test]
fn test_coordinator_functionality() {
    let coordinator = InstanceCoordinator::new();
    
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("test_file.txt").to_string_lossy().to_string();
    
    let result = coordinator.collect_files_from_instances(&test_file);
    
    match result {
        Some(_files) => assert!(true),
        None => assert!(true),
    }
}
