use std::fs::{self, OpenOptions};
use std::io::Write;
use std::thread;
use std::time::Duration;

/// Coordinates multiple instances of the application to process files together
pub struct InstanceCoordinator {
    temp_dir: std::path::PathBuf,
}

impl InstanceCoordinator {
    /// Create a new instance coordinator
    pub fn new() -> Self {
        Self {
            temp_dir: std::env::temp_dir(),
        }
    }

    /// Collect files from multiple instances, returning Some(files) if this is the first instance,
    /// or None if this is a subsequent instance (which should exit)
    pub fn collect_files_from_instances(&self, initial_file: &str) -> Option<Vec<String>> {
        let lock_file_path = self.temp_dir.join("jellyfin_rename.lock");
        
        // Try to acquire an exclusive lock by creating a new file
        let lock_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_file_path);
        
        let is_first_instance = lock_file.is_ok();
        
        if is_first_instance {
            self.handle_first_instance(lock_file, initial_file, &lock_file_path)
        } else {
            self.handle_subsequent_instance(initial_file);
            None
        }
    }

    /// Handle the first instance - coordinate with others and collect all files
    fn handle_first_instance(
        &self,
        lock_file: Result<std::fs::File, std::io::Error>,
        initial_file: &str,
        lock_file_path: &std::path::Path,
    ) -> Option<Vec<String>> {
        // We got the lock - write our process ID
        if let Ok(mut file) = lock_file {
            let _ = writeln!(file, "{}", std::process::id());
        }
        
        // Collect files from other instances
        let mut collected_files = vec![initial_file.to_string()];
        
        // Wait for other instances to start and signal their files
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_millis(1000);
        
        while start_time.elapsed() < timeout {
            thread::sleep(Duration::from_millis(50));
            
            // Check for signal files from other instances
            for i in 1..10 { // Check up to 10 instances
                let signal_file = self.temp_dir.join(format!("jellyfin_rename_signal_{}.tmp", i));
                if signal_file.exists() {
                    if let Ok(content) = fs::read_to_string(&signal_file) {
                        let file_path = content.trim().to_string();
                        if !collected_files.contains(&file_path) {
                            collected_files.push(file_path);
                        }
                    }
                    // Clean up the signal file
                    let _ = fs::remove_file(&signal_file);
                }
            }
        }
        
        // Clean up lock file when done
        let _ = fs::remove_file(lock_file_path);
        
        Some(collected_files)
    }

    /// Handle subsequent instances - signal our file and exit
    fn handle_subsequent_instance(&self, initial_file: &str) {
        let mut signal_file_created = false;
        
        // Try to create a signal file with a unique name
        for i in 1..10 {
            let signal_file = self.temp_dir.join(format!("jellyfin_rename_signal_{}.tmp", i));
            if let Ok(mut file) = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&signal_file) {
                
                let _ = write!(file, "{}", initial_file);
                signal_file_created = true;
                break;
            }
        }
        
        if !signal_file_created {
            // Fallback - try to append to a shared signal file
            let shared_signal = self.temp_dir.join("jellyfin_rename_shared_signal.tmp");
            let _ = fs::write(&shared_signal, initial_file);
        }
    }
}

impl Default for InstanceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let coordinator = InstanceCoordinator::new();
        assert!(coordinator.temp_dir.exists());
    }

    #[test]
    fn test_default_coordinator() {
        let coordinator = InstanceCoordinator::default();
        assert!(coordinator.temp_dir.exists());
    }
}
