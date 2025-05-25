use std::io::{self, stdout, Stdout};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use crate::rename_engine::{RenameOperation, RenameResult};

type Tui = Terminal<CrosstermBackend<Stdout>>;

struct App {
    rename_op: RenameOperation,
    input: String,
    input_mode: InputMode,
    confirmed: Option<bool>,
    show_preview: bool,
}

#[derive(Clone, Copy)]
enum InputMode {
    Normal,
    Editing,
}

impl App {    fn new(file_path: &str) -> Self {
        let rename_op = RenameOperation::new(file_path);
        let input = rename_op.get_name_without_extension().to_string();
        
        Self {
            rename_op,
            input,
            input_mode: InputMode::Editing,
            confirmed: None,
            show_preview: true,
        }
    }

    fn update_rename_op(&mut self) {
        self.rename_op.update_new_name(self.input.clone());
    }
}

fn init_terminal() -> io::Result<Tui> {
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;
    
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    
    Ok(terminal)
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn run_app(terminal: &mut Tui, mut app: App) -> io::Result<App> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // Use blocking event read instead of polling to avoid duplicates
        match event::read()? {
            Event::Key(key) => {                // Only handle key press events, ignore key release to prevent duplicates
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('e') => {
                                app.input_mode = InputMode::Editing;
                            }
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.confirmed = Some(false);
                                break;
                            }
                            KeyCode::Enter => {
                                app.update_rename_op();
                                app.confirmed = Some(true);
                                break;
                            }
                            _ => {}
                        },
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.update_rename_op();
                                    app.confirmed = Some(true);
                                    break;
                                } else {
                                    app.input_mode = InputMode::Normal;
                                }
                            }
                            KeyCode::Char(c) => {
                                app.input.push(c);
                                app.update_rename_op();
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                                app.update_rename_op();
                            }
                            KeyCode::Esc => {
                                app.confirmed = Some(false);
                                break;
                            }
                            KeyCode::Tab => {
                                app.show_preview = !app.show_preview;
                            }
                            _ => {}
                        },
                    }
                }
            }
            _ => {} // Ignore other events
        }
    }

    Ok(app)
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Create the main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Help
        ])
        .split(size);

    // Title
    let title = Paragraph::new("Jellyfin Rename Tool")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Main content area
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Original filename
            Constraint::Length(5),  // New filename input
            Constraint::Length(3),  // Preview
            Constraint::Min(3),     // Status/Instructions
        ])
        .margin(1)
        .split(chunks[1]);

    // Original filename
    let original_filename = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Original: ", Style::default().fg(Color::Yellow)),
            Span::raw(app.rename_op.get_original_name()),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Current File"))
    .wrap(Wrap { trim: true });
    f.render_widget(original_filename, main_chunks[0]);

    // New filename input
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(match app.input_mode {
            InputMode::Normal => "New Filename (Press 'e' to edit)",
            InputMode::Editing => "New Filename (Editing - Esc to cancel, Enter to confirm, Ctrl+Enter to rename)",
        });

    let input_style = match app.input_mode {
        InputMode::Normal => Style::default(),
        InputMode::Editing => Style::default().fg(Color::Yellow),
    };    let input_text = vec![
        Line::from(vec![
            Span::styled(&app.input, input_style),
            Span::raw(app.rename_op.get_extension()),
        ]),
    ];

    let input_paragraph = Paragraph::new(input_text)
        .style(input_style)
        .block(input_block);
    f.render_widget(input_paragraph, main_chunks[1]);

    // Set cursor position when editing
    if let InputMode::Editing = app.input_mode {
        f.set_cursor_position((
            main_chunks[1].x + app.input.len() as u16 + 1,
            main_chunks[1].y + 1,
        ));
    }

    // Preview
    if app.show_preview {        let preview_text = vec![
            Line::from(vec![
                Span::styled("Preview: ", Style::default().fg(Color::Green)),
                Span::raw(app.rename_op.get_new_name()),
            ]),
        ];
        let preview = Paragraph::new(preview_text)
            .block(Block::default().borders(Borders::ALL).title("Preview"))
            .wrap(Wrap { trim: true });
        f.render_widget(preview, main_chunks[2]);
    }

    // Help text
    let help_text = match app.input_mode {
        InputMode::Normal => {
            "Press 'e' to edit, Enter to rename, 'q' or Esc to cancel, Tab to toggle preview"
        }
        InputMode::Editing => {
            "Type to edit filename, Enter to stop editing, Ctrl+Enter to rename, Esc to cancel"
        }
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);

    // Show a popup if there's a status message
    if let Some(confirmed) = app.confirmed {
        let popup_area = centered_rect(60, 20, size);
        f.render_widget(Clear, popup_area);
        
        let message = if confirmed { "Renaming..." } else { "Cancelled" };
        let color = if confirmed { Color::Green } else { Color::Red };
        
        let popup = Paragraph::new(message)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(popup, popup_area);
    }
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

pub fn show_rename_dialog(file_path: &str) -> bool {
    // Initialize terminal
    let mut terminal = match init_terminal() {
        Ok(terminal) => terminal,
        Err(e) => {
            eprintln!("Failed to initialize terminal: {}", e);
            return false;
        }
    };

    // Create app and run it
    let app = App::new(file_path);
    let result_app = match run_app(&mut terminal, app) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Application error: {}", e);
            let _ = restore_terminal();
            return false;
        }
    };

    // Restore terminal before processing result
    if let Err(e) = restore_terminal() {
        eprintln!("Failed to restore terminal: {}", e);
    }

    // Process the result
    if let Some(confirmed) = result_app.confirmed {
        if confirmed {
            // Execute the rename operation
            let mut rename_op = RenameOperation::new(file_path);
            rename_op.update_new_name(result_app.input);
            
            match rename_op.execute() {
                RenameResult::Success(_) => {
                    println!("File renamed successfully!");
                    true
                }
                RenameResult::AlreadyExists => {
                    println!("Error: Target file already exists!");
                    false
                }
                RenameResult::NoPermission => {
                    println!("Error: No permission to rename file!");
                    false
                }
                RenameResult::SourceNotFound => {
                    println!("Error: Source file not found!");
                    false
                }
                RenameResult::OtherError(msg) => {
                    println!("Error: {}", msg);
                    false
                }
            }
        } else {
            println!("Rename cancelled.");
            false
        }
    } else {
        false
    }
}
