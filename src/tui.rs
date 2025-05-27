use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::fs;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame, Terminal,
};

use crate::rename_engine::{RenameEngine, FileRename, ConfigBuilder, extract_season_from_directory, extract_season_from_filename, FileType};

#[derive(Debug, Clone)]
pub struct FileItem {
    pub original_path: String,
    pub original_name: String,
    pub new_name: String,
    pub status: ProcessingStatus,
    pub error_message: Option<String>,
    pub episode_number: u32,
    pub episode_title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Success,
    Error,
    Skipped,
}

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
    pub year_input: String,
    pub imdb_id_input: String,
    pub use_imdb: bool,
    pub undo_operations: Vec<UndoOperation>, // Store undo operations
    pub needs_refresh: bool, // Flag to trigger refresh when season changes
    pub status_message: Option<String>, // Status message for user feedback
    pub status_message_time: Option<Instant>, // When the status message was set
    pub file_type: FileType, // Whether processing TV shows or movies
}

#[derive(Debug, PartialEq)]
pub enum ConfigInputMode {
    FileType,
    Directory,
    Season,
    Year,
    ImdbChoice,
    ImdbId,
    Confirm,
}

#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub total: usize,
    pub processed: usize,
    pub successful: usize,
    pub failed: usize,
}

#[derive(Debug, Clone)]
pub struct UndoOperation {
    pub original_path: String,
    pub renamed_path: String,
    pub original_name: String,
    pub new_name: String,
}

impl App {    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));        Self {
            files: Vec::new(),
            selected_index: 0,
            list_state,
            current_processing: None,
            processing_progress: 0.0,
            show_help: false,
            show_preview: true,
            show_config: true,            config_input_mode: ConfigInputMode::FileType,
            scroll_state: ScrollbarState::default(),
            start_time: None,
            finished: false,
            stats: ProcessingStats::default(),
            rename_engine: None,
            directory_input: String::new(),
            season_input: String::new(),
            year_input: String::new(),
            imdb_id_input: String::new(),            use_imdb: false,            undo_operations: Vec::new(),
            needs_refresh: false,
            status_message: None,
            status_message_time: None,
            file_type: FileType::TvShow, // Default to TV shows
        }
    }    pub fn with_directory(directory: String) -> Self {
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
                    // Try to detect season from filename if not already detected
                    if detected_season.is_none() {
                        detected_season = extract_season_from_filename(filename);
                    }
                    
                    files.push(FileItem {
                        original_path: file_path.clone(),
                        original_name: filename.to_string(),
                        new_name: filename.to_string(), // Will be updated during processing
                        status: ProcessingStatus::Pending,
                        error_message: None,
                        episode_number: 0, // Will be detected during processing
                        episode_title: String::new(),
                    });
                }
            }
        }
        
        app.files = files;
        app.stats.total = app.files.len();        // Set directory input from the first file's directory
        if let Some(dir) = directory {
            app.directory_input = dir.clone();
            
            // Don't auto-detect season until file type is selected
            // Season detection will happen when user selects TV shows
        }
        
        // Skip directory configuration if we have pre-selected files
        if !app.files.is_empty() {
            // For single file, skip to file type choice, then go to year
            // For multiple files, skip to file type choice then continue with full config
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
    }    pub async fn create_rename_engine(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            config.season(self.season_input.clone()) // This will set both season and season_num in the config
        } else {
            config
        };
        
        let config = if self.files.len() == 1 { 
            // For single files, year is required
            config.year(Some(self.year_input.clone()))
        } else if self.year_input.is_empty() { 
            // For multiple files, year is optional
            config.year(None)
        } else { 
            config.year(Some(self.year_input.clone()))
        };
        
        let config = if self.file_type == FileType::TvShow && self.files.len() > 1 && self.use_imdb && !self.imdb_id_input.is_empty() { 
            // Only enable IMDb for multiple TV show files
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
    }    pub fn toggle_preview(&mut self) {
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
        match self.config_input_mode {            ConfigInputMode::FileType => {
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
            }            ConfigInputMode::Season => {
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
            }            ConfigInputMode::Year => {
                if c == '\n' || c == '\r' {
                    self.advance_config_step();
                } else if c == '\x08' {
                    self.year_input.pop();
                    // Trigger refresh if we have selected files and input is being modified
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
                    }
                } else {
                    self.year_input.push(c);
                    // Trigger refresh if we have selected files and input is being modified
                    if !self.files.is_empty() {
                        self.needs_refresh = true;
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
    }pub fn advance_config_step(&mut self) {
        match self.config_input_mode {
            ConfigInputMode::FileType => {
                self.config_input_mode = ConfigInputMode::Directory;
            }
            ConfigInputMode::Directory => {
                if !self.directory_input.is_empty() {
                    // Check if we're doing TV shows or movies to determine next step
                    if self.file_type == FileType::TvShow {
                        // For TV shows, season is required
                        if self.files.len() == 1 {
                            // For single files, skip season input if auto-detected and go to year
                            if !self.season_input.is_empty() {
                                self.config_input_mode = ConfigInputMode::Year;
                            } else {
                                self.config_input_mode = ConfigInputMode::Season;
                            }
                        } else {
                            self.config_input_mode = ConfigInputMode::Season;
                        }
                    } else {
                        // For movies, skip season and go directly to year
                        self.config_input_mode = ConfigInputMode::Year;
                    }
                }
            }
            ConfigInputMode::Season => {
                if !self.season_input.is_empty() {
                    // Validate season input - it should be either "S##" format or just a number
                    let is_valid = if self.season_input.starts_with('S') || self.season_input.starts_with('s') {
                        let num_part = &self.season_input[1..];
                        num_part.parse::<u32>().is_ok()
                    } else {
                        self.season_input.parse::<u32>().is_ok()
                    };
                    
                    if is_valid {
                        // For multiple files, skip year input and go to IMDB choice
                        self.config_input_mode = ConfigInputMode::ImdbChoice;
                    }
                    // If invalid, stay in Season mode (user needs to fix it)
                }
            }            ConfigInputMode::Year => {
                // For movies, year is optional but recommended
                // For single TV episodes, year is required
                let year_required = self.file_type == FileType::TvShow && self.files.len() == 1;
                
                if year_required && self.year_input.is_empty() {
                    // Stay in Year mode if it's empty and required
                    return;
                }
                
                // Validate year input if not empty
                if !self.year_input.is_empty() {
                    if let Ok(year) = self.year_input.parse::<u32>() {
                        if year < 1900 || year > 2100 {
                            // Invalid year range, stay in Year mode
                            return;
                        }
                    } else {
                        // Not a valid number, stay in Year mode
                        return;
                    }
                }
                
                // For TV shows with multiple files, go to IMDb choice
                // For movies or single TV episodes, skip to confirm
                if self.file_type == FileType::TvShow && self.files.len() > 1 {
                    self.config_input_mode = ConfigInputMode::ImdbChoice;
                } else {
                    self.config_input_mode = ConfigInputMode::Confirm;
                }
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
                self.processing_progress = (index as f64) / (total_files as f64);                // Create FileRename from FileItem
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
            // Process each pre-selected file
            for file_item in &mut self.files {
                let path = std::path::Path::new(&file_item.original_path);
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {                    // Try to process with standard pattern first
                    if let Some(file_rename) = engine.process_file_standard(filename)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                    } else if let Some(file_rename) = engine.process_file_flexible(filename)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                    } else if let Some(file_rename) = engine.process_file_movie(filename)? {
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
    }    pub async fn refresh_selected_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    }    pub async fn undo_renames(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
        
        // Reset processing state
        self.finished = false;
        self.current_processing = None;
        self.processing_progress = 0.0;
        self.stats.successful = 0;
        self.stats.failed = 0;
        self.stats.processed = 0;
        
        // Ensure list selection is valid
        if !self.files.is_empty() {
            let selected = self.list_state.selected().unwrap_or(0);
            if selected >= self.files.len() {
                self.list_state.select(Some(0));
                self.selected_index = 0;
            }
        }
        
        // Set status message based on results
        if undo_errors.is_empty() {
            self.set_status_message(format!("Successfully undid {} rename operations", successful_undos));
        } else {
            self.set_status_message(format!("Undid {} operations with {} errors", successful_undos, undo_errors.len()));
        }
        
        Ok(())
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

pub async fn run_tui(directory: Option<String>, selected_files: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = if !selected_files.is_empty() {
        App::with_selected_files(selected_files)
    } else if let Some(dir) = directory {
        App::with_directory(dir)
    } else {
        App::new()
    };
    
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        // Clear expired status messages
        app.clear_status_message_if_expired();
        
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.show_help {
                                app.toggle_help();
                            } else if app.show_config {
                                return Ok(());
                            } else {
                                return Ok(());
                            }
                        }
                        KeyCode::Char('h') => app.toggle_help(),
                        KeyCode::Char('p') => {
                            if !app.show_config {
                                app.toggle_preview();
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.show_config {
                                app.next();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.show_config {
                                app.previous();
                            }
                        }
                        KeyCode::Enter => {
                            if app.show_config {
                                if app.config_input_mode == ConfigInputMode::Confirm {
                                    // Create engine
                                    if let Err(_e) = app.create_rename_engine().await {
                                        // Show error
                                        continue;
                                    }
                                    
                                    // Process files based on whether they were pre-selected or scanned
                                    if !app.files.is_empty() {
                                        // Files were pre-selected, process them
                                        if let Err(_e) = app.process_selected_files().await {
                                            // Show error
                                            continue;
                                        }
                                    } else {
                                        // Scan directory for files
                                        if let Err(_e) = app.scan_directory().await {
                                            // Show error
                                            continue;
                                        }
                                    }
                                } else {
                                    app.advance_config_step();
                                }                        } else if !app.finished {
                                let _ = app.process_files().await;
                            }
                        }
                        KeyCode::Char('u') => {
                            // Undo renames if finished and have undo operations
                            if app.finished && !app.undo_operations.is_empty() && !app.show_config {
                                let _ = app.undo_renames().await;
                            }
                        }                        KeyCode::Char(c) => {
                            if app.show_config {
                                app.handle_config_input(c);
                            }
                        }
                        KeyCode::Backspace => {
                            if app.show_config {
                                app.handle_config_input('\x08');
                            }
                        }
                        _ => {}
                    }
                }
            }        }        // Handle refresh flag for season/year changes
        if app.needs_refresh && app.show_config {
            app.needs_refresh = false;
            // Only refresh if we have valid input to avoid infinite refresh
            let should_refresh = match app.config_input_mode {
                ConfigInputMode::Season => !app.season_input.is_empty(),
                ConfigInputMode::Year => true, // Year can be empty for single files
                _ => false,
            };
            
            if should_refresh {
                let _ = app.refresh_selected_files().await;
            }
        }

        if app.finished {
            // Keep showing the UI after completion
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    if app.show_config {
        render_config_screen(f, size, app);
    } else {
        render_main_screen(f, size, app);
    }

    // Help popup (if enabled)
    if app.show_help {
        render_help_popup(f, app);
    }
}

fn render_config_screen(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let header = Paragraph::new("Jellyfin Rename Tool - Configuration")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(header, chunks[0]);    // Configuration form - adjust constraints based on file count and file type
    let has_multiple_files = app.files.len() > 1;
    let is_tv_show = app.file_type == FileType::TvShow;
    
    let mut form_constraints = vec![
        Constraint::Length(3), // File type
        Constraint::Length(3), // Directory
    ];
    
    // Add constraints based on file type and count
    if is_tv_show {
        form_constraints.push(Constraint::Length(3)); // Season (for TV shows)
    }
    
    if (is_tv_show && app.files.len() == 1) || (!is_tv_show) {
        form_constraints.push(Constraint::Length(3)); // Year (required for single TV episodes or movies)
    }
      if is_tv_show && has_multiple_files {
        form_constraints.push(Constraint::Length(3)); // IMDb choice (for multiple TV episodes)
        if app.use_imdb || app.config_input_mode == ConfigInputMode::ImdbId {
            form_constraints.push(Constraint::Length(3)); // IMDb ID
        }
    }
    
    form_constraints.push(Constraint::Length(3)); // Confirm
    form_constraints.push(Constraint::Min(1));    // Remaining space
    
    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(form_constraints)
        .split(chunks[1]);

    let mut current_chunk_index = 0;

    // File type selection
    let file_type_text = if app.config_input_mode == ConfigInputMode::FileType {
        "Press T for TV Shows, M for Movies"
    } else {
        match app.file_type {
            FileType::TvShow => "TV Shows",
            FileType::Movie => "Movies",
        }
    };
    
    let file_type_input = Paragraph::new(file_type_text)
        .style(if app.config_input_mode == ConfigInputMode::FileType {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("File Type")
                .border_style(if app.config_input_mode == ConfigInputMode::FileType {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }),
        );
    f.render_widget(file_type_input, form_chunks[current_chunk_index]);
    current_chunk_index += 1;

    // Directory input
    let directory_style = if app.config_input_mode == ConfigInputMode::Directory {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let directory_input = Paragraph::new(app.directory_input.as_str())
        .style(directory_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Directory Path")
                .border_style(if app.config_input_mode == ConfigInputMode::Directory {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                }),
        );    f.render_widget(directory_input, form_chunks[current_chunk_index]);
    current_chunk_index += 1;    if is_tv_show {
        // Season input (only for TV shows)
        let season_style = if app.config_input_mode == ConfigInputMode::Season {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        
        let season_title = if !app.season_input.is_empty() {
            "Season (auto-detected) - Press Enter to continue or edit"
        } else {
            "Season (REQUIRED - e.g., S01 or 1)"
        };
        
        let season_display = if app.season_input.is_empty() {
            "[Enter season number]".to_string()
        } else {
            app.season_input.clone()
        };
        
        let season_input = Paragraph::new(season_display.as_str())
            .style(season_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(season_title)
                    .border_style(if app.config_input_mode == ConfigInputMode::Season {
                        if app.season_input.is_empty() {
                            Style::default().fg(Color::Red) // Red border if empty and focused
                        } else {
                            Style::default().fg(Color::Green) // Green border if auto-detected and focused
                        }
                    } else {
                        if app.season_input.is_empty() {
                            Style::default().fg(Color::Red) // Red border if empty
                        } else {
                            Style::default().fg(Color::Green) // Green border if filled
                        }
                    }),
            );
        f.render_widget(season_input, form_chunks[current_chunk_index]);
        current_chunk_index += 1;
    }

    // Year input logic
    let show_year = (is_tv_show && app.files.len() == 1) || !is_tv_show;
    if show_year {
        let year_style = if app.config_input_mode == ConfigInputMode::Year {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        
        let year_title = if is_tv_show && app.files.len() == 1 {
            if app.year_input.is_empty() {
                "Year (REQUIRED for single TV episodes)"
            } else {
                "Year"
            }
        } else {
            "Year (optional for movies)"
        };
        
        let year_display = if app.year_input.is_empty() {
            "[Enter year]".to_string()
        } else {
            app.year_input.clone()
        };
        
        let year_required = is_tv_show && app.files.len() == 1;
        
        let year_input = Paragraph::new(year_display.as_str())
            .style(year_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(year_title)
                    .border_style(if app.config_input_mode == ConfigInputMode::Year {
                        if app.year_input.is_empty() && year_required {
                            Style::default().fg(Color::Red) // Red border if empty and required
                        } else {
                            Style::default().fg(Color::Yellow) // Yellow border if focused
                        }
                    } else {
                        if app.year_input.is_empty() && year_required {
                            Style::default().fg(Color::Red) // Red border if empty and required
                        } else {
                            Style::default().fg(Color::Green) // Green border if filled or optional
                        }
                    }),
            );
        f.render_widget(year_input, form_chunks[current_chunk_index]);
        current_chunk_index += 1;
    }    // IMDb choice (only for TV shows with multiple files)
    if is_tv_show && has_multiple_files {
        let imdb_text = if app.config_input_mode == ConfigInputMode::ImdbChoice {
            "Press y for Yes, n for No"
        } else if app.use_imdb {
            "Yes"
        } else {
            "No"
        };
        
        let imdb_choice = Paragraph::new(imdb_text)
            .style(if app.config_input_mode == ConfigInputMode::ImdbChoice {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Use IMDb for episode titles?")
                    .border_style(if app.config_input_mode == ConfigInputMode::ImdbChoice {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Gray)
                    }),
            );
        f.render_widget(imdb_choice, form_chunks[current_chunk_index]);
        current_chunk_index += 1;
    }    // IMDb ID input (if needed and only for TV shows with multiple files)
    if is_tv_show && has_multiple_files && (app.use_imdb || app.config_input_mode == ConfigInputMode::ImdbId) {
        let imdb_style = if app.config_input_mode == ConfigInputMode::ImdbId {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        
        let imdb_input = Paragraph::new(app.imdb_id_input.as_str())
            .style(imdb_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("IMDb ID (e.g., tt0944947)")
                    .border_style(if app.config_input_mode == ConfigInputMode::ImdbId {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Gray)
                    }),
            );
        f.render_widget(imdb_input, form_chunks[current_chunk_index]);
        current_chunk_index += 1;
    }

    // Confirm button
    if app.config_input_mode == ConfigInputMode::Confirm {
        let confirm_text = if app.files.is_empty() {
            "Press ENTER to scan directory and start"
        } else {
            "Press ENTER to process selected files"
        };
        
        let confirm = Paragraph::new(confirm_text)
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Ready to Process")
                    .border_style(Style::default().fg(Color::Green)),
            );
        f.render_widget(confirm, form_chunks[current_chunk_index]);
    }    // Instructions
    let instructions = match app.config_input_mode {
        ConfigInputMode::FileType => "Choose file type: T for TV Shows, M for Movies",
        ConfigInputMode::Directory => "Enter the directory path containing your video files",
        ConfigInputMode::Season => {
            if app.season_input.is_empty() {
                "Season number is REQUIRED (e.g., S01, S1, 1, or 01)"
            } else {
                "Season auto-detected! Press Enter to continue or type to edit"
            }
        },
        ConfigInputMode::Year => {
            if app.file_type == FileType::TvShow && app.files.len() == 1 {
                "Year is REQUIRED for single TV episodes (e.g., 2023)"
            } else {
                "Enter year or leave blank (press Enter to skip)"
            }
        },
        ConfigInputMode::ImdbChoice => "Would you like to fetch episode titles from IMDb?",
        ConfigInputMode::ImdbId => "Enter the IMDb series ID (found in the URL)",
        ConfigInputMode::Confirm => "Review your settings and press Enter to continue",
    };

    let help_text = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Instructions"));

    f.render_widget(help_text, chunks[2]);
}

fn render_main_screen(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Create main layout
    let chunks = if app.show_preview {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(area)
    };

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(chunks[0]);

    // Header
    render_header(f, left_chunks[0], app);

    // File list
    render_file_list(f, left_chunks[1], app);

    // Status bar
    render_status_bar(f, left_chunks[2], app);

    // Preview panel (if enabled)
    if app.show_preview && chunks.len() > 1 {
        render_preview_panel(f, chunks[1], app);
    }
}

fn render_header(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let title = if app.finished {
        "Jellyfin Rename Tool - Completed!"
    } else if app.current_processing.is_some() {
        "Jellyfin Rename Tool - Processing..."
    } else {
        "Jellyfin Rename Tool"
    };

    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(header, area);
}

fn render_file_list(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let (icon, color) = match file.status {
                ProcessingStatus::Pending => ("[PENDING]", Color::Yellow),
                ProcessingStatus::Processing => ("[PROCESSING]", Color::Blue),
                ProcessingStatus::Success => ("[SUCCESS]", Color::Green),
                ProcessingStatus::Error => ("[ERROR]", Color::Red),
                ProcessingStatus::Skipped => ("[SKIPPED]", Color::Gray),
            };

            let line = if app.current_processing == Some(i) {
                Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::styled(
                        file.original_name.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::styled(file.original_name.clone(), Style::default().fg(Color::White)),
                ])
            };

            ListItem::new(line)
        })
        .collect();

    let files_list = List::new(items)
        .block(
            Block::default()
                .title("Files to Process")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(files_list, area, &mut app.list_state.clone());

    // Render scrollbar
    if app.files.len() > area.height as usize - 2 {        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut app.scroll_state.clone(),
        );
    }
}

fn render_status_bar(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(area);

    // Progress bar
    let progress_label = if let Some(status_msg) = &app.status_message {
        // Show status message instead of progress when available
        status_msg.clone()
    } else if app.finished {
        format!("Complete! {} successful, {} failed", app.stats.successful, app.stats.failed)
    } else if app.current_processing.is_some() {
        format!("Processing... {}/{}", app.stats.processed + 1, app.stats.total)
    } else {
        format!("Ready to process {} files", app.stats.total)
    };

    let progress_style = if app.status_message.is_some() {
        Style::default().fg(Color::Cyan) // Use different color for status messages
    } else {
        Style::default().fg(Color::Green)
    };

    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(progress_style)
        .percent((app.processing_progress * 100.0) as u16)
        .label(progress_label);

    f.render_widget(progress, chunks[0]);// Controls hint
    let controls_text = if app.finished && !app.undo_operations.is_empty() {
        "Press u to undo, h for help, q to quit"
    } else {
        "Press ENTER to start, h for help, q to quit"
    };
    
    let controls = Paragraph::new(controls_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Controls"));

    f.render_widget(controls, chunks[1]);
}

fn render_preview_panel(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    if let Some(selected) = app.list_state.selected() {
        if let Some(file) = app.files.get(selected) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(area);

            // Original filename
            let original = Paragraph::new(Text::from(vec![
                Line::from("Original:"),
                Line::from(Span::styled(
                    file.original_name.clone(),
                    Style::default().fg(Color::Red),
                )),
                Line::from(""),
                Line::from(format!("Episode: {}", file.episode_number)),
                Line::from(format!("Title: {}", file.episode_title)),
            ]))
            .block(
                Block::default()
                    .title("Before")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .wrap(Wrap { trim: true });

            f.render_widget(original, chunks[0]);

            // New filename
            let new_style = match file.status {
                ProcessingStatus::Success => Style::default().fg(Color::Green),
                ProcessingStatus::Error => Style::default().fg(Color::Red),
                _ => Style::default().fg(Color::Yellow),
            };

            let mut new_lines = vec![
                Line::from("New:"),
                Line::from(Span::styled(file.new_name.clone(), new_style)),
            ];

            if let Some(error) = &file.error_message {
                new_lines.push(Line::from(""));
                new_lines.push(Line::from(Span::styled(
                    format!("Error: {}", error),
                    Style::default().fg(Color::Red),
                )));
            }

            let new = Paragraph::new(Text::from(new_lines))
                .block(
                    Block::default()
                        .title("After")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                )
                .wrap(Wrap { trim: true });

            f.render_widget(new, chunks[1]);
        }
    }
}

fn render_help_popup(f: &mut Frame, _app: &App) {
    let popup_area = centered_rect(60, 50, f.area());

    let help_text = vec![
        Line::from(vec![
            Span::styled("Jellyfin Rename Tool - Help", Style::default().add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),        Line::from("Navigation:"),
        Line::from("  Up/k    - Move up"),
        Line::from("  Down/j  - Move down"),
        Line::from(""),        Line::from("Actions:"),
        Line::from("  Enter   - Start processing"),
        Line::from("  Space   - Start processing"),
        Line::from("  u       - Undo renames (after completion)"),
        Line::from("  p       - Toggle preview panel"),
        Line::from("  h/F1    - Toggle this help"),
        Line::from("  q/Esc   - Quit application"),
        Line::from(""),
        Line::from("Features:"),
        Line::from("• Fetches episode titles from IMDb"),
        Line::from("• Removes common torrent site tags"),
        Line::from("• Cleans up video quality indicators"),
        Line::from("• Removes codec information"),
        Line::from("• Preserves original file structure"),
        Line::from("• Supports multiple filename patterns"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press Esc or h to close", Style::default().fg(Color::Gray))
        ]),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}