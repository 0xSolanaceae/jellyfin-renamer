use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::fs;

use crate::rename_engine::{RenameEngine, FileRename};
use super::app::App;
use super::models::{FileItem, ProcessingStatus, UndoOperation};

impl App {
    // File processing methods
    pub async fn process_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(engine) = &self.rename_engine {
            self.start_time = Some(Instant::now());
            let total_files = self.files.len();
            
            for index in 0..total_files {
                self.current_processing = Some(index);
                self.files[index].status = ProcessingStatus::Processing;
                self.processing_progress = (index as f64) / (total_files as f64);

                // Create FileRename from FileItem
                let file_rename = FileRename {
                    original_path: PathBuf::from(&self.files[index].original_path),
                    original_name: self.files[index].original_name.clone(),
                    new_name: self.files[index].new_name.clone(),
                    episode_number: self.files[index].episode_number,
                    season_number: 1, // Default season for processing
                    episode_title: self.files[index].episode_title.clone(),
                    needs_rename: self.files[index].original_name != self.files[index].new_name,
                };

                // Skip files that don't need renaming
                if !file_rename.needs_rename {
                    self.files[index].status = ProcessingStatus::Skipped;
                    self.stats.processed += 1;
                    continue;
                }

                let result = engine.rename_file(&file_rename).await;
                
                if result.success {
                    self.files[index].status = ProcessingStatus::Success;
                    self.stats.successful += 1;
                    
                    // Track successful rename for undo
                    let new_path = PathBuf::from(&self.files[index].original_path)
                        .parent()
                        .unwrap()
                        .join(&self.files[index].new_name);
                    
                    self.undo_operations.push(UndoOperation {
                        original_path: self.files[index].original_path.clone(),
                        renamed_path: new_path.to_string_lossy().to_string(),
                        original_name: self.files[index].original_name.clone(),
                        new_name: self.files[index].new_name.clone(),
                    });
                } else {
                    self.files[index].status = ProcessingStatus::Error;
                    self.files[index].error_message = result.error_message;
                    self.stats.failed += 1;
                }
                
                self.stats.processed += 1;

                // Small delay to show progress
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            self.current_processing = None;
            self.processing_progress = 1.0;
            self.finished = true;
        }
        Ok(())
    }

    pub async fn process_selected_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(engine) = &self.rename_engine {
            // Store files length before mutable iteration to avoid borrow checker issues
            let files_len = self.files.len();
            
            // Process each pre-selected file
            for (index, file_item) in self.files.iter_mut().enumerate() {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    // For multiple movies, use individual years
                    let file_year = if self.file_type == crate::rename_engine::FileType::Movie && files_len > 1 {
                        if index < self.movie_years.len() && !self.movie_years[index].is_empty() {
                            Some(self.movie_years[index].clone())
                        } else {
                            None
                        }
                    } else {
                        // For single files or TV shows, use global year
                        if self.year_input.is_empty() { None } else { Some(self.year_input.clone()) }
                    };
                    
                    // Process with individual year if needed
                    if let Some(file_rename) = engine.process_file_with_year(filename, file_year)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                    }
                    // If no pattern matches, keep original name
                }
            }

            if !self.files.is_empty() {
                self.list_state.select(Some(0));
                self.show_config = false;
            }
        }
        Ok(())
    }

    pub async fn refresh_selected_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Only refresh if we have selected files and a rename engine
        if self.files.is_empty() || self.rename_engine.is_none() {
            return Ok(());
        }

        // Ensure season input is properly formatted before processing
        if !self.season_input.is_empty() && !self.season_input.starts_with('S') && !self.season_input.starts_with('s') {
            // Convert raw number to S format (e.g., "2" to "S02")
            if let Ok(season_num) = self.season_input.parse::<u32>() {
                self.season_input = format!("S{:02}", season_num);
            }
        }

        // Parse manual season number from user input
        let manual_season_num = self.season_input.trim_start_matches("S").trim_start_matches("s").parse::<u32>().unwrap_or(1);
        
        // Recreate the rename engine with the current inputs
        self.create_rename_engine().await?;

        if let Some(engine) = &self.rename_engine {
            // Reprocess each file with the updated season
            for file_item in &mut self.files {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    // Reset to original state first
                    file_item.new_name = file_item.original_name.clone();
                    file_item.episode_number = 0;
                    file_item.episode_title = String::new();
                    file_item.status = ProcessingStatus::Pending;

                    // Process with manual season override
                    if let Some(file_rename) = engine.process_file_with_manual_season(filename, manual_season_num)? {
                        // Update file item with values from the rename result
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                        
                        // Check if rename is actually needed
                        file_item.status = if file_rename.needs_rename { 
                            ProcessingStatus::Pending 
                        } else { 
                            ProcessingStatus::Skipped 
                        };
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn undo_renames(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.undo_operations.is_empty() {
            return Ok(());
        }

        self.set_status_message("Undoing renames...".to_string());
        
        // Reverse the undo operations to restore original state
        for undo_op in self.undo_operations.iter().rev() {
            if let Err(e) = fs::rename(&undo_op.renamed_path, &undo_op.original_path) {
                eprintln!("Failed to undo rename: {}", e);
                continue;
            }
        }

        // Clear undo operations
        self.undo_operations.clear();
        
        // Reset file statuses and refresh with original names
        if let Some(engine) = &self.rename_engine {
            for file_item in &mut self.files {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    // Reset to pending status
                    file_item.status = ProcessingStatus::Pending;
                    file_item.error_message = None;
                    
                    // Reprocess to get the correct new names
                    if self.file_type == crate::rename_engine::FileType::TvShow {
                        // For TV shows, try standard processing first
                        if let Some(file_rename) = engine.process_file(filename)? {
                            file_item.new_name = file_rename.new_name;
                            file_item.episode_number = file_rename.episode_number;
                            file_item.episode_title = file_rename.episode_title;
                            file_item.status = if file_rename.needs_rename { ProcessingStatus::Pending } else { ProcessingStatus::Skipped };
                        } else if let Some(file_rename) = engine.process_file_flexible(filename)? {
                            file_item.new_name = file_rename.new_name;
                            file_item.episode_number = file_rename.episode_number;
                            file_item.episode_title = file_rename.episode_title;
                            file_item.status = if file_rename.needs_rename { ProcessingStatus::Pending } else { ProcessingStatus::Skipped };
                        }
                    } else if let Some(file_rename) = engine.process_file_movie(filename)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                        file_item.status = if file_rename.needs_rename { ProcessingStatus::Pending } else { ProcessingStatus::Skipped };
                    }
                }
            }
        }

        self.finished = false;
        self.stats.processed = 0;
        self.stats.successful = 0;
        self.stats.failed = 0;
        
        self.set_status_message("Renames undone successfully!".to_string());
        
        Ok(())
    }
}
