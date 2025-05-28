use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::fs;
use ratatui::widgets::{ListState, ScrollbarState};
use crossterm::event::KeyCode;
use tokio;

use crate::rename_engine::{
    RenameEngine, FileRename, ConfigBuilder, 
    extract_season_from_directory, extract_season_from_filename, FileType
};
use super::models::{FileItem, ProcessingStatus, ConfigInputMode, ProcessingStats, UndoOperation};

#[derive(Debug)]
pub struct App {
    pub files: Vec<FileItem>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub current_processing: Option<usize>,
    pub processing_progress: f64,
    pub show_help: bool,
    pub show_preview: bool,
    pub show_config: bool,
    pub config_input_mode: ConfigInputMode,
    pub scroll_state: ScrollbarState,
    pub start_time: Option<Instant>,
    pub finished: bool,
    pub stats: ProcessingStats,
    pub rename_engine: Option<RenameEngine>,
    pub directory_input: String,
    pub season_input: String,
    pub year_input: String,    pub movie_years: Vec<String>,
    pub current_movie_index: usize,
    pub imdb_id_input: String,
    pub use_imdb: bool,
    pub undo_operations: Vec<UndoOperation>,
    pub needs_refresh: bool,
    pub status_message: Option<String>,
    pub status_message_time: Option<Instant>,
    pub file_type: FileType,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        Self {
            files: Vec::new(),
            selected_index: 0,
            list_state,
            current_processing: None,
            processing_progress: 0.0,
            show_help: false,
            show_preview: true,
            show_config: true,
            config_input_mode: ConfigInputMode::FileType,
            scroll_state: ScrollbarState::default(),
            start_time: None,
            finished: false,
            stats: ProcessingStats::default(),
            rename_engine: None,
            directory_input: String::new(),
            season_input: String::new(),
            year_input: String::new(),
            movie_years: Vec::new(),
            current_movie_index: 0,
            imdb_id_input: String::new(),
            use_imdb: false,
            undo_operations: Vec::new(),
            needs_refresh: false,
            status_message: None,
            status_message_time: None,
            file_type: FileType::TvShow, // Default to TV shows
        }
    }

    pub fn with_directory(directory: String) -> Self {
        let mut app = Self::new();
        app.directory_input = directory.clone();
        
        // Don't auto-detect season until file type is selected
        // Season detection will happen when user selects TV shows
        
        app
    }

    pub fn with_selected_files(selected_files: Vec<String>) -> Self {
        let mut app = Self::new();
        
        // Convert selected file paths to FileItems
        let mut files = Vec::new();
        let mut directory = None;
        let mut detected_season = None;
        
        for file_path in selected_files {
            let path = std::path::Path::new(&file_path);
            if path.is_file() {
                // Get directory from first file
                if directory.is_none() {
                    directory = path.parent().map(|p| p.to_string_lossy().to_string());
                }
                  if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    if detected_season.is_none() {
                        detected_season = extract_season_from_filename(filename);
                    }
                    
                    files.push(FileItem {
                        original_path: file_path.clone(),
                        original_name: filename.to_string(),
                        new_name: filename.to_string(),
                        status: ProcessingStatus::Pending,
                        error_message: None,
                        episode_number: 0,
                        episode_title: String::new(),
                    });
                }
            }
        }
          app.files = files;
        app.stats.total = app.files.len();
        
        app.movie_years = vec![String::new(); app.files.len()];
        
        if let Some(dir) = directory {
            app.directory_input = dir.clone();
        }
        
        if !app.files.is_empty() {
            app.config_input_mode = ConfigInputMode::FileType;
        }
        
        app
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

            self.stats = ProcessingStats {
                total: self.files.len(),
                ..Default::default()
            };

            if !self.files.is_empty() {
                self.list_state.select(Some(0));
                self.show_config = false;
            }
        }
        Ok(())
    }

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

    pub fn next(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.files.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected_index = i;
    }

    pub fn previous(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.files.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected_index = i;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
    }

    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_message_time = Some(Instant::now());
    }

    pub fn clear_status_message_if_expired(&mut self) {
        if let (Some(_), Some(time)) = (&self.status_message, self.status_message_time) {
            if time.elapsed() > Duration::from_secs(3) {
                self.status_message = None;
                self.status_message_time = None;
            }
        }
    }    pub fn handle_config_input(&mut self, c: char) {
        match self.config_input_mode {
            ConfigInputMode::FileType => {
                if c == 't' || c == 'T' {
                    self.file_type = FileType::TvShow;
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
                } else if c == '\x08' {
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
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
                    }
                } else {
                    self.season_input.push(c);
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
                    }
                }
            }
            ConfigInputMode::Year => {
                if c == '\n' || c == '\r' {
                    if !self.year_input.is_empty() {
                        if let Ok(year) = self.year_input.parse::<u32>() {
                            if year >= 1900 && year <= 2100 {
                                self.advance_config_step();
                            }
                        }
                    } else {
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
            }            ConfigInputMode::MovieYears => {
                if c == '\n' || c == '\r' {
                    let current_year = &self.movie_years[self.current_movie_index];
                    if !current_year.is_empty() {
                        if let Ok(year) = current_year.parse::<u32>() {
                            if year < 1900 || year > 2100 {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    
                    if self.current_movie_index < self.files.len() - 1 {
                        self.current_movie_index += 1;
                    } else {
                        self.advance_config_step();
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
                // For TV shows, never go to Year - go directly to IMDb choice or Confirm
                if self.files.len() > 1 {
                    self.config_input_mode = ConfigInputMode::ImdbChoice;
                } else {
                    self.config_input_mode = ConfigInputMode::Confirm;
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
            }            ConfigInputMode::Confirm => {
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
                // Year is only for movies now
                if !self.files.is_empty() {
                    self.config_input_mode = ConfigInputMode::FileType;
                } else {
                    self.config_input_mode = ConfigInputMode::Directory;
                }
            }
            ConfigInputMode::MovieYears => {
                if !self.files.is_empty() {
                    self.config_input_mode = ConfigInputMode::FileType;
                } else {
                    self.config_input_mode = ConfigInputMode::Directory;
                }
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
                } else if self.file_type == FileType::TvShow && self.files.len() == 1 {
                    self.config_input_mode = ConfigInputMode::Season;
                } else if self.file_type == FileType::Movie && self.files.len() > 1 {
                    self.config_input_mode = ConfigInputMode::MovieYears;
                } else {
                    self.config_input_mode = ConfigInputMode::Year;
                }
            }
            ConfigInputMode::FileType => {
                // Can't go back from first step
            }
        }
    }

    pub fn handle_config_navigation(&mut self, key: KeyCode) {        match key {
            KeyCode::Left | KeyCode::Backspace => {
                if self.config_input_mode == ConfigInputMode::MovieYears {
                    if self.current_movie_index > 0 {
                        self.current_movie_index -= 1;
                    } else {
                        self.go_back_config_step();
                    }
                } else {
                    self.go_back_config_step();
                }
            }
            KeyCode::Right => {
                if self.config_input_mode == ConfigInputMode::MovieYears {
                    if self.current_movie_index < self.files.len() - 1 {
                        self.current_movie_index += 1;
                    }
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

    pub async fn process_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(engine) = &self.rename_engine {
            self.start_time = Some(Instant::now());
            let total_files = self.files.len();
            
            for index in 0..total_files {
                self.current_processing = Some(index);
                self.files[index].status = ProcessingStatus::Processing;
                self.processing_progress = (index as f64) / (total_files as f64);                let file_rename = FileRename {
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
                    let file_year = if self.file_type == FileType::Movie && files_len > 1 {
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

        let mut undo_errors = Vec::new();
        let mut successful_undos = 0;

        for undo_op in self.undo_operations.iter().rev() {
            match fs::rename(&undo_op.renamed_path, &undo_op.original_path) {
                Ok(_) => {
                    successful_undos += 1;
                }
                Err(e) => {
                    undo_errors.push(format!("Failed to undo {}: {}", undo_op.new_name, e));
                }
            }
        }

        // Clear undo operations after performing undo
        self.undo_operations.clear();
        
        // Reset ALL file statuses and names properly (not just successful ones)
        for file in &mut self.files {
            // Reset status to pending for all files that were processed
            if file.status == ProcessingStatus::Success || file.status == ProcessingStatus::Error || file.status == ProcessingStatus::Skipped {
                file.status = ProcessingStatus::Pending;
            }
            // Reset new_name back to original_name for all files
            file.new_name = file.original_name.clone();
            // Clear episode info for all files
            file.episode_number = 0;
            file.episode_title.clear();
            file.error_message = None;
        }
        
        // Reprocess files with the rename engine to recalculate new names
        if let Some(engine) = &self.rename_engine {
            for file_item in &mut self.files {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    // Try different processing methods to recalculate new names
                    if let Some(file_rename) = engine.process_file_standard(filename)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                        file_item.status = if file_rename.needs_rename { ProcessingStatus::Pending } else { ProcessingStatus::Skipped };
                    } else if let Some(file_rename) = engine.process_file_flexible(filename)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                        file_item.status = if file_rename.needs_rename { ProcessingStatus::Pending } else { ProcessingStatus::Skipped };
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
        self.current_processing = None;
        self.processing_progress = 0.0;
        self.stats.successful = 0;
        self.stats.failed = 0;
        self.stats.processed = 0;
        
        if !self.files.is_empty() {
            let selected = self.list_state.selected().unwrap_or(0);
            if selected >= self.files.len() {
                self.list_state.select(Some(0));
                self.selected_index = 0;
            }
        }
        
        if undo_errors.is_empty() {
            self.set_status_message(format!("Successfully undid {} rename operations", successful_undos));
        } else {
            self.set_status_message(format!("Undid {} operations with {} errors", successful_undos, undo_errors.len()));
        }
        
        Ok(())
    }    pub fn auto_detect_season_for_tv_shows(&mut self) {
        if self.file_type != FileType::TvShow {
            return;
        }
        
        let mut detected_season = None;
        
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
        
        if detected_season.is_none() && !self.directory_input.is_empty() {
            if let Some(dir_path) = std::path::Path::new(&self.directory_input).file_name() {
                if let Some(dir_name) = dir_path.to_str() {
                    detected_season = extract_season_from_directory(dir_name);
                }
            }
            
            if detected_season.is_none() {
                if let Some(parent_path) = std::path::Path::new(&self.directory_input).parent() {
                    if let Some(parent_dir) = parent_path.file_name().and_then(|f| f.to_str()) {
                        detected_season = extract_season_from_directory(parent_dir);
                    }
                }
            }
        }
        
        if let Some(season_num) = detected_season {
            self.season_input = format!("S{:02}", season_num);
        } else if self.season_input.is_empty() {
            self.season_input = "S01".to_string();
        }
    }
}
