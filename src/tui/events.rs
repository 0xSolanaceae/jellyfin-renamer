use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Frame, Terminal,
};

use super::app::App;
use super::models::ConfigInputMode;
use super::rendering::ui;

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
                            } else {
                                app.handle_config_navigation(KeyCode::Down);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.show_config {
                                app.previous();
                            } else {
                                app.handle_config_navigation(KeyCode::Up);
                            }
                        }
                        KeyCode::Left => {
                            if app.show_config {
                                app.handle_config_navigation(KeyCode::Left);
                            }
                        }
                        KeyCode::Right => {
                            if app.show_config {
                                app.handle_config_navigation(KeyCode::Right);
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
                                }
                            } else if !app.finished {
                                let _ = app.process_files().await;
                            }
                        }
                        KeyCode::Char('u') => {
                            // Undo renames if finished and have undo operations
                            if app.finished && !app.undo_operations.is_empty() && !app.show_config {
                                let _ = app.undo_renames().await;
                            }
                        }
                        KeyCode::Char(c) => {
                            if app.show_config {
                                app.handle_config_input(c);
                            }
                        }
                        KeyCode::Backspace => {
                            if app.show_config {
                                // Handle backspace for navigation or text input
                                match app.config_input_mode {
                                    ConfigInputMode::Directory | 
                                    ConfigInputMode::Season | 
                                    ConfigInputMode::Year | 
                                    ConfigInputMode::MovieYears | 
                                    ConfigInputMode::ImdbId => {
                                        app.handle_config_input('\x08');
                                    }
                                    _ => {
                                        app.handle_config_navigation(KeyCode::Backspace);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Handle refresh flag for season/year changes
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
