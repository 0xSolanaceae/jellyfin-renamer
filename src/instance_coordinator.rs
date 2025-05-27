use std::fs::{self, OpenOptions};
use std::io::{Write, Read};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashSet;

/// Coordinates multiple instances of the application to process files together
pub struct InstanceCoordinator {
    temp_dir: std::path::PathBuf,
    app_id: String,
    session_id: String,
}

impl InstanceCoordinator {
    pub fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let session_id = format!("{}_{}", timestamp, std::process::id());
        
        Self {
            temp_dir: std::env::temp_dir(),
            app_id: "jellyfin_rename".to_string(),
            session_id,
        }
    }    pub fn collect_files_from_instances(&self, initial_file: &str) -> Option<Vec<String>> {
        let base_path = self.temp_dir.join(&self.app_id);
        
        let _ = fs::create_dir_all(&base_path);
        
        let lock_file_path = base_path.join("coordinator.lock");
        let files_dir = base_path.join("files");
        let _ = fs::create_dir_all(&files_dir);
        
        self.add_file_to_collection(&files_dir, initial_file);
        
        match self.try_become_coordinator(&lock_file_path) {
            Some(_) => {
                self.handle_coordinator_instance(&files_dir, &lock_file_path)
            }
            None => {
                None
            }
        }
    }    fn try_become_coordinator(&self, lock_file_path: &std::path::Path) -> Option<()> {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_file_path) 
        {
            Ok(mut lock_file) => {
                let _ = writeln!(lock_file, "{}:{}", self.session_id, std::process::id());
                Some(())
            }
            Err(_) => {
                if let Ok(mut file) = std::fs::File::open(lock_file_path) {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        if let Some(first_line) = contents.lines().next() {
                            if let Some((_session, pid_str)) = first_line.split_once(':') {
                                if let Ok(pid) = pid_str.parse::<u32>() {
                                    if !self.is_process_running(pid) {
                                        let _ = fs::remove_file(lock_file_path);
                                        return self.try_become_coordinator(lock_file_path);
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
        }
    }    fn is_process_running(&self, pid: u32) -> bool {
        use std::process::Command;
        
        match Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/FO", "CSV"])
            .output() 
        {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.lines().count() > 1
            }
            Err(_) => true
        }
    }    fn add_file_to_collection(&self, files_dir: &std::path::Path, file_path: &str) {
        let file_id = format!("{}.txt", self.session_id);
        let file_entry_path = files_dir.join(&file_id);
        
        for attempt in 0..10 {
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_entry_path) 
            {
                Ok(mut file) => {
                    if writeln!(file, "{}", file_path).is_ok() {
                        break;
                    }
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(10 * (attempt + 1)));
                }
            }
        }
    }    fn handle_coordinator_instance(
        &self,
        files_dir: &std::path::Path,
        lock_file_path: &std::path::Path,
    ) -> Option<Vec<String>> {
        let mut collected_files = HashSet::new();
        
        let start_time = Instant::now();
        let max_wait_time = Duration::from_millis(5000);
        let mut last_file_count = 0;
        let mut stable_count = 0;
        
        println!("Collecting files from multiple instances...");
        
        while start_time.elapsed() < max_wait_time {
            thread::sleep(Duration::from_millis(100));
            
            if let Ok(entries) = fs::read_dir(files_dir) {
                collected_files.clear();
                
                for entry in entries.flatten() {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        for line in content.lines() {
                            let line = line.trim();
                            if !line.is_empty() && std::path::Path::new(line).exists() {
                                collected_files.insert(line.to_string());
                            }
                        }
                    }
                }
                
                if collected_files.len() != last_file_count {
                    println!("Found {} files so far...", collected_files.len());
                }
                
                if collected_files.len() == last_file_count {
                    stable_count += 1;
                    if stable_count >= 10 {
                        break;
                    }
                } else {
                    stable_count = 0;
                    last_file_count = collected_files.len();
                }
            }
        }
        
        let _ = fs::remove_file(lock_file_path);
        let _ = fs::remove_dir_all(files_dir);
        
        let final_files: Vec<String> = collected_files.into_iter().collect();
        println!("Collected {} files total", final_files.len());
        
        Some(final_files)
    }
}

impl Default for InstanceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
