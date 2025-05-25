use std::io;
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

use crate::rename_engine;

#[derive(Debug, Clone)]
pub struct FileItem {
    pub original_path: String,
    pub original_name: String,
    pub new_name: String,
    pub status: ProcessingStatus,
    pub error_message: Option<String>,
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
    pub scroll_state: ScrollbarState,
    pub start_time: Option<Instant>,
    pub finished: bool,
    pub stats: ProcessingStats,
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
    pub fn new(file_paths: Vec<String>) -> Self {
        let mut files = Vec::new();
        
        for path in file_paths {
            let original_name = std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            
            let new_name = rename_engine::process_filename(&original_name);
            
            files.push(FileItem {
                original_path: path,
                original_name: original_name.clone(),
                new_name,
                status: ProcessingStatus::Pending,
                error_message: None,
            });
        }

        let mut list_state = ListState::default();
        if !files.is_empty() {
            list_state.select(Some(0));
        }

        let stats = ProcessingStats {
            total: files.len(),
            ..Default::default()
        };

        Self {
            files,
            selected_index: 0,
            list_state,
            current_processing: None,
            processing_progress: 0.0,
            show_help: false,
            show_preview: true,
            scroll_state: ScrollbarState::default(),
            start_time: None,
            finished: false,
            stats,
        }
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
    }    pub async fn process_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start_time = Some(Instant::now());
        let total_files = self.files.len();
        
        for index in 0..total_files {
            self.current_processing = Some(index);
            self.files[index].status = ProcessingStatus::Processing;
            self.processing_progress = (index as f64) / (total_files as f64);

            // Simulate processing time for demo purposes
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Actually process the file
            let file_path = self.files[index].original_path.clone();
            let success = rename_engine::rename_file(&file_path);
            
            if success {
                self.files[index].status = ProcessingStatus::Success;
                self.stats.successful += 1;
            } else {
                self.files[index].status = ProcessingStatus::Error;
                self.files[index].error_message = Some("Failed to rename file".to_string());
                self.stats.failed += 1;
            }
            
            self.stats.processed += 1;
        }

        self.current_processing = None;
        self.processing_progress = 1.0;
        self.finished = true;
        Ok(())
    }
}

pub async fn run_tui(file_paths: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(file_paths);
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
                            } else {
                                return Ok(());
                            }
                        }
                        KeyCode::Char('h') => app.toggle_help(),
                        KeyCode::Char('p') => app.toggle_preview(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if !processing && !app.finished {
                                processing = true;
                                let _ = app.process_files().await;
                                processing = false;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.finished {
            // Keep showing the UI for a moment after completion
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.size();

    // Create main layout
    let chunks = if app.show_preview {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(size)
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

    // Help popup (if enabled)
    if app.show_help {
        render_help_popup(f, app);
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
        Line::from("• Removes common torrent site tags"),
        Line::from("• Cleans up video quality indicators"),
        Line::from("• Removes codec information"),
        Line::from("• Preserves original file structure"),
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