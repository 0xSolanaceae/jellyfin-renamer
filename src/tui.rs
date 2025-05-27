use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};
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

use crate::rename_engine::{RenameConfig, RenameEngine, FileRename, ConfigBuilder, extract_season_from_directory, extract_season_from_filename};

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
    pub config_input: String,
    pub scroll_state: ScrollbarState,
    pub start_time: Option<Instant>,
    pub finished: bool,
    pub stats: ProcessingStats,
    pub config: Option<RenameConfig>,
    pub rename_engine: Option<RenameEngine>,
    pub directory_input: String,
    pub season_input: String,
    pub year_input: String,
    pub imdb_id_input: String,
    pub use_imdb: bool,
}

#[derive(Debug, PartialEq)]
pub enum ConfigInputMode {
    Directory,
    Season,
    Year,
    ImdbChoice,
    ImdbId,
    Confirm,
    None,
}

#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub total: usize,
    pub processed: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped: usize,
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
            config_input_mode: ConfigInputMode::Directory,
            config_input: String::new(),
            scroll_state: ScrollbarState::default(),
            start_time: None,
            finished: false,
            stats: ProcessingStats::default(),
            config: None,
            rename_engine: None,
            directory_input: String::new(),
            season_input: String::new(),
            year_input: String::new(),
            imdb_id_input: String::new(),
            use_imdb: false,
        }
    }    pub fn with_directory(directory: String) -> Self {
        let mut app = Self::new();
        app.directory_input = directory.clone();
        
        // Try to auto-detect season from directory name
        if let Some(dir_path) = std::path::Path::new(&directory).file_name() {
            if let Some(dir_name) = dir_path.to_str() {
                if let Some(season_num) = extract_season_from_directory(dir_name) {
                    app.season_input = format!("S{:02}", season_num);
                }
            }
        }
        
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
        app.stats.total = app.files.len();
        
        // Set directory input from the first file's directory
        if let Some(dir) = directory {
            app.directory_input = dir.clone();
            
            // Try to auto-detect season from directory name if not detected from filename
            if detected_season.is_none() {
                if let Some(dir_path) = std::path::Path::new(&dir).file_name() {
                    if let Some(dir_name) = dir_path.to_str() {
                        detected_season = extract_season_from_directory(dir_name);
                    }
                }
            }
            
            // Also try parent directory if still not found
            if detected_season.is_none() {
                if let Some(parent_path) = std::path::Path::new(&dir).parent() {
                    if let Some(parent_dir) = parent_path.file_name().and_then(|f| f.to_str()) {
                        detected_season = extract_season_from_directory(parent_dir);
                    }
                }
            }
        }
        
        // Set detected season if found
        if let Some(season_num) = detected_season {
            app.season_input = format!("S{:02}", season_num);
        }
        
        // Skip directory configuration if we have pre-selected files
        if !app.files.is_empty() {
            app.config_input_mode = ConfigInputMode::Season;
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
                status: ProcessingStatus::Pending,
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
        let config = ConfigBuilder::new()
            .directory(&self.directory_input)
            .season(self.season_input.clone())
            .year(if self.year_input.is_empty() { None } else { Some(self.year_input.clone()) })
            .imdb(if self.use_imdb && !self.imdb_id_input.is_empty() { 
                Some(self.imdb_id_input.clone()) 
            } else { 
                None 
            })
            .build()?;

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

    pub fn handle_config_input(&mut self, c: char) {
        match self.config_input_mode {
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
                } else {
                    self.season_input.push(c);
                }
            }
            ConfigInputMode::Year => {
                if c == '\n' || c == '\r' {
                    self.advance_config_step();
                } else if c == '\x08' {
                    self.year_input.pop();
                } else {
                    self.year_input.push(c);
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
            ConfigInputMode::Directory => {
                if !self.directory_input.is_empty() {
                    self.config_input_mode = ConfigInputMode::Season;
                }
            }            ConfigInputMode::Season => {
                if !self.season_input.is_empty() {
                    // Validate season input - it should be either "S##" format or just a number
                    let is_valid = if self.season_input.starts_with('S') || self.season_input.starts_with('s') {
                        let num_part = &self.season_input[1..];
                        num_part.parse::<u32>().is_ok()
                    } else {
                        self.season_input.parse::<u32>().is_ok()
                    };
                    
                    if is_valid {
                        self.config_input_mode = ConfigInputMode::Year;
                    }
                    // If invalid, stay in Season mode (user needs to fix it)
                }
            }
            ConfigInputMode::Year => {
                self.config_input_mode = ConfigInputMode::ImdbChoice;
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
                self.processing_progress = (index as f64) / (total_files as f64);

                // Create FileRename from FileItem
                let file_rename = FileRename {
                    original_path: PathBuf::from(&self.files[index].original_path),
                    original_name: self.files[index].original_name.clone(),
                    new_name: self.files[index].new_name.clone(),
                    episode_number: self.files[index].episode_number,
                    episode_title: self.files[index].episode_title.clone(),
                };

                let result = engine.rename_file(&file_rename).await;
                
                if result.success {
                    self.files[index].status = ProcessingStatus::Success;
                    self.stats.successful += 1;
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
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    // Try to process with standard pattern first
                    if let Some(file_rename) = engine.process_file_standard(filename)? {
                        file_item.new_name = file_rename.new_name;
                        file_item.episode_number = file_rename.episode_number;
                        file_item.episode_title = file_rename.episode_title;
                    } else if let Some(file_rename) = engine.process_file_flexible(filename)? {
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
    let mut processing = false;
    
    loop {
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
                        KeyCode::Enter => {                            if app.show_config {
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
                                }
                            } else if !processing && !app.finished {
                                processing = true;
                                let _ = app.process_files().await;
                                processing = false;
                            }
                        }
                        KeyCode::Char(c) => {
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
            }
        }

        if app.finished {
            // Keep showing the UI after completion
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.size();

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
    let header = Paragraph::new("🔧 Jellyfin Rename Tool - Configuration")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(header, chunks[0]);

    // Configuration form
    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(chunks[1]);

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
        );
    f.render_widget(directory_input, form_chunks[0]);    // Season input
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
    f.render_widget(season_input, form_chunks[1]);

    // Year input
    let year_style = if app.config_input_mode == ConfigInputMode::Year {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let year_input = Paragraph::new(app.year_input.as_str())
        .style(year_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Year (optional)")
                .border_style(if app.config_input_mode == ConfigInputMode::Year {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                }),
        );
    f.render_widget(year_input, form_chunks[2]);

    // IMDb choice
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
    f.render_widget(imdb_choice, form_chunks[3]);

    // IMDb ID input (if needed)
    if app.use_imdb || app.config_input_mode == ConfigInputMode::ImdbId {
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
        f.render_widget(imdb_input, form_chunks[4]);
    }

    // Confirm button
    if app.config_input_mode == ConfigInputMode::Confirm {
        let confirm = Paragraph::new("Press ENTER to scan directory and start")
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Ready to Scan")
                    .border_style(Style::default().fg(Color::Green)),
            );
        f.render_widget(confirm, form_chunks[5]);
    }    // Instructions
    let instructions = match app.config_input_mode {
        ConfigInputMode::Directory => "Enter the directory path containing your video files",
        ConfigInputMode::Season => {
            if app.season_input.is_empty() {
                "Season number is REQUIRED (e.g., S01, S1, 1, or 01)"
            } else {
                "Season auto-detected! Press Enter to continue or type to edit"
            }
        },
        ConfigInputMode::Year => "Enter year or leave blank (press Enter to skip)",
        ConfigInputMode::ImdbChoice => "Would you like to fetch episode titles from IMDb?",
        ConfigInputMode::ImdbId => "Enter the IMDb series ID (found in the URL)",
        ConfigInputMode::Confirm => "Review your settings and press Enter to continue",
        ConfigInputMode::None => "",
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
        "🎉 Jellyfin Rename Tool - Completed!"
    } else if app.current_processing.is_some() {
        "⚡ Jellyfin Rename Tool - Processing..."
    } else {
        "📁 Jellyfin Rename Tool"
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
                ProcessingStatus::Pending => ("⏳", Color::Yellow),
                ProcessingStatus::Processing => ("⚡", Color::Blue),
                ProcessingStatus::Success => ("✅", Color::Green),
                ProcessingStatus::Error => ("❌", Color::Red),
                ProcessingStatus::Skipped => ("⏭️", Color::Gray),
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
        .highlight_symbol("► ");

    f.render_stateful_widget(files_list, area, &mut app.list_state.clone());

    // Render scrollbar
    if app.files.len() > area.height as usize - 2 {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        f.render_stateful_widget(
            scrollbar,
            area.inner(&Margin {
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
    let progress_label = if app.finished {
        format!("Complete! {} successful, {} failed", app.stats.successful, app.stats.failed)
    } else if app.current_processing.is_some() {
        format!("Processing... {}/{}", app.stats.processed + 1, app.stats.total)
    } else {
        format!("Ready to process {} files", app.stats.total)
    };

    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Green))
        .percent((app.processing_progress * 100.0) as u16)
        .label(progress_label);

    f.render_widget(progress, chunks[0]);

    // Controls hint
    let controls = Paragraph::new("Press ENTER to start, h for help, q to quit")
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
    let popup_area = centered_rect(60, 50, f.size());

    let help_text = vec![
        Line::from(vec![
            Span::styled("Jellyfin Rename Tool - Help", Style::default().add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/k     - Move up"),
        Line::from("  ↓/j     - Move down"),
        Line::from(""),
        Line::from("Actions:"),
        Line::from("  Enter   - Start processing"),
        Line::from("  Space   - Start processing"),
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