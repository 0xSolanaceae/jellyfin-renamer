use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::fs;

use crate::rename_engine::{RenameEngine, FileRename};
use super::app::App;
use super::models::{FileItem, ProcessingStatus, UndoOperation};

impl App {
    pub async fn process_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(engine) = &self.rename_engine {
            self.start_time = Some(Instant::now());
            let total_files = self.files.len();
            
            for index in 0..total_files {
                self.current_processing = Some(index);
                self.files[index].status = ProcessingStatus::Processing;
                self.processing_progress = (index as f64) / (total_files as f64);

                let file_rename = FileRename {
                    original_path: PathBuf::from(&self.files[index].original_path),
                    original_name: self.files[index].original_name.clone(),
                    new_name: self.files[index].new_name.clone(),
                    episode_number: self.files[index].episode_number,
                    season_number: 1,
                    episode_title: self.files[index].episode_title.clone(),
                    needs_rename: self.files[index].original_name != self.files[index].new_name,
                };

                if !file_rename.needs_rename {
                    self.files[index].status = ProcessingStatus::Skipped;
                    self.stats.processed += 1;
                    continue;
                }

                let result = engine.rename_file(&file_rename).await;
                  if result.success {
                    self.files[index].status = ProcessingStatus::Success;
                    self.stats.successful += 1;
                    
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

                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            self.current_processing = None;
            self.processing_progress = 1.0;
            self.finished = true;
        }
        Ok(())
    }    pub async fn process_selected_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(engine) = &self.rename_engine {
            let files_len = self.files.len();
            
            for (index, file_item) in self.files.iter_mut().enumerate() {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    let file_year = if self.file_type == crate::rename_engine::FileType::Movie && files_len > 1 {
                        if index < self.movie_years.len() && !self.movie_years[index].is_empty() {
                            Some(self.movie_years[index].clone())
                        } else {
                            None
                        }
                    } else {
                        if self.year_input.is_empty() { None } else { Some(self.year_input.clone()) }
                    };
                      if let Some(file_rename) = engine.process_file_with_year(filename, file_year)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                    }
                }
            }            // Sort files by episode number for TV shows
            if self.file_type == crate::rename_engine::FileType::TvShow {
                self.sort_files_by_episode();
            }

            if !self.files.is_empty() {
                self.list_state.select(Some(0));
                self.show_config = false;
            }
        }
        Ok(())
    }pub async fn refresh_selected_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.files.is_empty() || self.rename_engine.is_none() {
            return Ok(());
        }

        if !self.season_input.is_empty() && !self.season_input.starts_with('S') && !self.season_input.starts_with('s') {
            if let Ok(season_num) = self.season_input.parse::<u32>() {
                self.season_input = format!("S{:02}", season_num);
            }
        }

        let manual_season_num = self.season_input.trim_start_matches("S").trim_start_matches("s").parse::<u32>().unwrap_or(1);
        
        self.create_rename_engine().await?;

        if let Some(engine) = &self.rename_engine {
            // Reprocess each file with the updated season
            for file_item in &mut self.files {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    // Reset to original state first
                    file_item.new_name = file_item.original_name.clone();
                    file_item.episode_number = 0;
                    file_item.episode_title = String::new();                    file_item.status = ProcessingStatus::Pending;

                    if let Some(file_rename) = engine.process_file_with_manual_season(filename, manual_season_num)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;                        file_item.episode_title = file_rename.episode_title;
                        
                        file_item.status = if file_rename.needs_rename { 
                            ProcessingStatus::Pending 
                        } else { 
                            ProcessingStatus::Skipped 
                        };
                    }                }
            }
        }

        // Sort files by episode number for TV shows
        if self.file_type == crate::rename_engine::FileType::TvShow {
            self.sort_files_by_episode();
        }

        Ok(())
    }

    pub async fn undo_renames(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.undo_operations.is_empty() {
            return Ok(());
        }        self.set_status_message("Undoing renames...".to_string());
        
        for undo_op in self.undo_operations.iter().rev() {
            if let Err(e) = fs::rename(&undo_op.renamed_path, &undo_op.original_path) {
                eprintln!("Failed to undo rename: {}", e);
                continue;
            }
        }

        self.undo_operations.clear();
        
        if let Some(engine) = &self.rename_engine {
            for file_item in &mut self.files {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    file_item.status = ProcessingStatus::Pending;
                    file_item.error_message = None;
                      if self.file_type == crate::rename_engine::FileType::TvShow {
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
