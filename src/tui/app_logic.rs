use std::path::PathBuf;
use crossterm::event::KeyCode;

use crate::rename_engine::{
    RenameEngine, FileRename, ConfigBuilder, 
    extract_season_from_directory, extract_season_from_filename, FileType
};
use super::app::App;
use super::models::{FileItem, ProcessingStatus, ConfigInputMode, UndoOperation};

impl App {
    // Configuration and engine methods
    pub async fn create_rename_engine(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure season input is properly formatted for TV shows
        if self.file_type == FileType::TvShow {
            if !self.season_input.starts_with('S') && !self.season_input.starts_with('s') {
                // Convert raw number to S format (e.g., "2" to "S02")
                if let Ok(season_num) = self.season_input.parse::<u32>() {
                    self.season_input = format!("S{:02}", season_num);
                }
            }
        }
        
        let config = ConfigBuilder::new()
            .directory(&self.directory_input)
            .file_type(self.file_type.clone());
        
        let config = if self.file_type == FileType::TvShow {
            config.season(self.season_input.clone())
        } else {
            config
        };
        
        // For single files (TV or movie), use the single year input
        // For multiple movies, we'll handle individual years during processing
        let config = if self.files.len() == 1 { 
            config.year(if self.year_input.is_empty() { None } else { Some(self.year_input.clone()) })
        } else if self.file_type == FileType::TvShow && !self.year_input.is_empty() { 
            config.year(Some(self.year_input.clone()))
        } else { 
            config.year(None)
        };
        
        let config = if self.file_type == FileType::TvShow && self.files.len() > 1 && self.use_imdb && !self.imdb_id_input.is_empty() { 
            config.imdb(Some(self.imdb_id_input.clone()))
        } else { 
            config.imdb(None)
        };
        
        let config = config.build()?;

        let mut engine = RenameEngine::new(config)?;
        engine.fetch_imdb_titles().await?;
        
        self.rename_engine = Some(engine);
        Ok(())
    }

    pub async fn scan_directory(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(engine) = &self.rename_engine {
            let file_renames = engine.scan_directory()?;
            self.files = file_renames.into_iter().map(|fr| FileItem {
                original_path: fr.original_path.to_string_lossy().to_string(),
                original_name: fr.original_name.clone(),
                new_name: fr.new_name.clone(),
                status: if fr.needs_rename { ProcessingStatus::Pending } else { ProcessingStatus::Skipped },
                error_message: None,
                episode_number: fr.episode_number,
                episode_title: fr.episode_title.clone(),
            }).collect();

            self.stats.total = self.files.len();
            self.stats.processed = 0;
            self.stats.successful = 0;
            self.stats.failed = 0;

            if !self.files.is_empty() {
                self.list_state.select(Some(0));
                self.show_config = false;
            }
        }
        Ok(())
    }

    // Configuration input handling
    pub fn handle_config_input(&mut self, c: char) {
        match self.config_input_mode {
            ConfigInputMode::FileType => {
                if c == 't' || c == 'T' {
                    self.file_type = FileType::TvShow;
                    // Auto-detect season when TV shows are selected
                    self.auto_detect_season_for_tv_shows();
                    self.advance_config_step();
                } else if c == 'm' || c == 'M' {
                    self.file_type = FileType::Movie;
                    self.advance_config_step();
                }
            }
            ConfigInputMode::Directory => {
                if c == '\n' || c == '\r' {
                    self.advance_config_step();
                } else if c == '\x08' { // Backspace
                    self.directory_input.pop();
                } else {
                    self.directory_input.push(c);
                }
            }
            ConfigInputMode::Season => {
                if c == '\n' || c == '\r' {
                    self.advance_config_step();
                } else if c == '\x08' {
                    self.season_input.pop();
                    // Trigger refresh if we have selected files and input is being modified
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
                    }
                } else {
                    self.season_input.push(c);
                    // Trigger refresh if we have selected files and input is being modified
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
                    }
                }
            }
            ConfigInputMode::Year => {
                if c == '\n' || c == '\r' {
                    // Validate year before advancing
                    if !self.year_input.is_empty() {
                        if let Ok(year) = self.year_input.parse::<u32>() {
                            if year >= 1900 && year <= 2100 {
                                self.advance_config_step();
                            }
                            // If invalid year, stay in Year mode (don't advance)
                        }
                        // If not a valid number, stay in Year mode
                    } else {
                        // Empty year is allowed, advance
                        self.advance_config_step();
                    }
                } else if c == '\x08' {
                    self.year_input.pop();
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
                    }
                } else if c.is_ascii_digit() {
                    self.year_input.push(c);
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
                    }
                }
            }
            ConfigInputMode::MovieYears => {
                if c == '\n' || c == '\r' {
                    // Validate current movie year before advancing
                    if self.current_movie_index < self.movie_years.len() {
                        let current_year = &self.movie_years[self.current_movie_index];
                        if !current_year.is_empty() {
                            if let Ok(year) = current_year.parse::<u32>() {
                                if year < 1900 || year > 2100 {
                                    // Invalid year, stay on this movie
                                    return;
                                }
                            } else {
                                // Not a valid number, stay on this movie
                                return;
                            }
                        }
                        
                        // Move to next movie or advance to next step
                        if self.current_movie_index < self.files.len() - 1 {
                            self.current_movie_index += 1;
                        } else {
                            self.advance_config_step();
                        }
                    }
                } else if c == '\x08' {
                    if self.current_movie_index < self.movie_years.len() {
                        self.movie_years[self.current_movie_index].pop();
                        if !self.files.is_empty() {
                            self.needs_refresh = true;
                        }
                    }
                } else if c.is_ascii_digit() {
                    if self.current_movie_index < self.movie_years.len() {
                        self.movie_years[self.current_movie_index].push(c);
                        if !self.files.is_empty() {
                            self.needs_refresh = true;
                        }
                    }
                }
            }
            ConfigInputMode::ImdbChoice => {
                if c == 'y' || c == 'Y' {
                    self.use_imdb = true;
                    self.advance_config_step();
                } else if c == 'n' || c == 'N' {
                    self.use_imdb = false;
                    self.advance_config_step();
                }
            }
            ConfigInputMode::ImdbId => {
                if c == '\n' || c == '\r' {
                    self.advance_config_step();
                } else if c == '\x08' {
                    self.imdb_id_input.pop();
                } else {
                    self.imdb_id_input.push(c);
                }
            }
            _ => {}
        }
    }

    pub fn advance_config_step(&mut self) {
        match self.config_input_mode {
            ConfigInputMode::FileType => {
                // Skip directory if we have pre-selected files
                if !self.files.is_empty() {
                    if self.file_type == FileType::TvShow {
                        self.config_input_mode = ConfigInputMode::Season;
                    } else {
                        // For movies with multiple files, go to MovieYears
                        if self.files.len() > 1 {
                            self.config_input_mode = ConfigInputMode::MovieYears;
                        } else {
                            self.config_input_mode = ConfigInputMode::Year;
                        }
                    }
                } else {
                    self.config_input_mode = ConfigInputMode::Directory;
                }
            }
            ConfigInputMode::Directory => {
                if self.file_type == FileType::TvShow {
                    self.config_input_mode = ConfigInputMode::Season;
                } else {
                    self.config_input_mode = ConfigInputMode::Year;
                }
            }
            ConfigInputMode::Season => {
                // For single TV episodes, go to Year
                if self.files.len() == 1 {
                    self.config_input_mode = ConfigInputMode::Year;
                } else {
                    // For multiple TV episodes, go to IMDb choice
                    self.config_input_mode = ConfigInputMode::ImdbChoice;
                }
            }
            ConfigInputMode::Year => {
                self.config_input_mode = ConfigInputMode::Confirm;
            }
            ConfigInputMode::MovieYears => {
                self.config_input_mode = ConfigInputMode::Confirm;
            }
            ConfigInputMode::ImdbChoice => {
                if self.use_imdb {
                    self.config_input_mode = ConfigInputMode::ImdbId;
                } else {
                    self.config_input_mode = ConfigInputMode::Confirm;
                }
            }
            ConfigInputMode::ImdbId => {
                self.config_input_mode = ConfigInputMode::Confirm;
            }
            ConfigInputMode::Confirm => {
                // Stay in confirm mode - handled elsewhere
            }
        }
    }

    pub fn go_back_config_step(&mut self) {
        match self.config_input_mode {
            ConfigInputMode::Directory => {
                self.config_input_mode = ConfigInputMode::FileType;
            }
            ConfigInputMode::Season => {
                if !self.files.is_empty() {
                    self.config_input_mode = ConfigInputMode::FileType;
                } else {
                    self.config_input_mode = ConfigInputMode::Directory;
                }
            }
            ConfigInputMode::Year => {
                if self.file_type == FileType::TvShow {
                    self.config_input_mode = ConfigInputMode::Season;
                } else {
                    if !self.files.is_empty() {
                        self.config_input_mode = ConfigInputMode::FileType;
                    } else {
                        self.config_input_mode = ConfigInputMode::Directory;
                    }
                }
            }
            ConfigInputMode::MovieYears => {
                if !self.files.is_empty() {
                    self.config_input_mode = ConfigInputMode::FileType;
                } else {
                    self.config_input_mode = ConfigInputMode::Directory;
                }
                // Reset to first movie when going back
                self.current_movie_index = 0;
            }
            ConfigInputMode::ImdbChoice => {
                self.config_input_mode = ConfigInputMode::Season;
            }
            ConfigInputMode::ImdbId => {
                self.config_input_mode = ConfigInputMode::ImdbChoice;
            }
            ConfigInputMode::Confirm => {
                if self.file_type == FileType::TvShow && self.files.len() > 1 {
                    if self.use_imdb {
                        self.config_input_mode = ConfigInputMode::ImdbId;
                    } else {
                        self.config_input_mode = ConfigInputMode::ImdbChoice;
                    }
                } else if self.file_type == FileType::Movie && self.files.len() > 1 {
                    self.config_input_mode = ConfigInputMode::MovieYears;
                } else {
                    self.config_input_mode = ConfigInputMode::Year;
                }
            }
            _ => {} // FileType has no previous step
        }
    }

    pub fn handle_config_navigation(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left | KeyCode::Backspace => {
                self.go_back_config_step();
            }
            KeyCode::Right | KeyCode::Enter => {
                if self.config_input_mode != ConfigInputMode::Confirm {
                    self.advance_config_step();
                }
            }
            KeyCode::Up => {
                if self.config_input_mode == ConfigInputMode::MovieYears {
                    if self.current_movie_index > 0 {
                        self.current_movie_index -= 1;
                    }
                }
            }
            KeyCode::Down => {
                if self.config_input_mode == ConfigInputMode::MovieYears {
                    if self.current_movie_index < self.files.len() - 1 {
                        self.current_movie_index += 1;
                    }
                }
            }
            _ => {}
        }
    }

    // Auto-detect season information when TV shows are selected
    pub fn auto_detect_season_for_tv_shows(&mut self) {
        if self.file_type != FileType::TvShow {
            return;
        }
        
        let mut detected_season = None;
        
        // Try to detect season from selected files first
        if !self.files.is_empty() {
            for file in &self.files {
                if let Some(filename) = std::path::Path::new(&file.original_path).file_name().and_then(|f| f.to_str()) {
                    if let Some(season_num) = extract_season_from_filename(filename) {
                        detected_season = Some(season_num);
                        break;
                    }
                }
            }
        }
        
        // If no season detected from files, try directory name
        if detected_season.is_none() && !self.directory_input.is_empty() {
            if let Some(dir_path) = std::path::Path::new(&self.directory_input).file_name() {
                if let Some(dir_name) = dir_path.to_str() {
                    detected_season = extract_season_from_directory(dir_name);
                }
            }
            
            // Also try parent directory if still not found
            if detected_season.is_none() {
                if let Some(parent_path) = std::path::Path::new(&self.directory_input).parent() {
                    if let Some(parent_dir) = parent_path.file_name().and_then(|f| f.to_str()) {
                        detected_season = extract_season_from_directory(parent_dir);
                    }
                }
            }
        }
        
        // Set detected season or default to S01
        if let Some(season_num) = detected_season {
            self.season_input = format!("S{:02}", season_num);
        } else if self.season_input.is_empty() {
            self.season_input = "S01".to_string();
        }
    }
}
