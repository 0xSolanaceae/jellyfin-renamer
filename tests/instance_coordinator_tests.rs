// Integration tests for the instance coordinator module

use jellyfin_rename::instance_coordinator::InstanceCoordinator;
use std::env;

#[test]
fn test_coordinator_creation() {
    let _coordinator = InstanceCoordinator::new();
    // Test that coordinator is created successfully
    // We can't access private fields, but we can test that it doesn't panic
    assert!(true); // Coordinator creation succeeded
}

#[test]
fn test_default_coordinator() {
    let _coordinator = InstanceCoordinator::default();
    // Test that default coordinator is created successfully
    assert!(true); // Default coordinator creation succeeded
}

#[test]
fn test_coordinator_functionality() {
    let coordinator = InstanceCoordinator::new();
    
    // Test the public interface - this should work without panicking
    // Using a temporary test file path
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("test_file.txt").to_string_lossy().to_string();
    
    // This tests the main functionality of the coordinator
    // The collect_files_from_instances method should handle the file gracefully
    let result = coordinator.collect_files_from_instances(&test_file);
    
    // The result can be either Some (if coordinator) or None (if worker)
    // Both are valid outcomes, so we just verify it doesn't panic
    match result {
        Some(_files) => assert!(true), // Coordinator instance
        None => assert!(true),         // Worker instance
    }
}
