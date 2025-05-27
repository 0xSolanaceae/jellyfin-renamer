use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

use crate::rename_engine::FileType;
use super::app::App;
use super::models::{ProcessingStatus, ConfigInputMode};
use super::utils::centered_rect;

pub fn ui(f: &mut Frame, app: &App) {
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

pub fn render_config_screen(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5), // Increased height for instructions
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
    f.render_widget(header, chunks[0]);

    // Configuration form - adjust constraints based on file count and file type
    let has_multiple_files = app.files.len() > 1;
    let is_tv_show = app.file_type == FileType::TvShow;
    let is_multiple_movies = app.file_type == FileType::Movie && has_multiple_files;
    
    let mut form_constraints = vec![
        Constraint::Length(3), // File type
        Constraint::Length(3), // Directory
    ];
    
    // Add constraints based on file type and count
    if is_tv_show {
        form_constraints.push(Constraint::Length(3)); // Season (for TV shows)
    }
    
    if (is_tv_show && app.files.len() == 1) || (app.file_type == FileType::Movie && app.files.len() == 1) {
        form_constraints.push(Constraint::Length(3)); // Year (for single files)
    }
    
    if is_multiple_movies {
        form_constraints.push(Constraint::Length(5)); // Movie years (multiple movies)
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
    let directory_input = Paragraph::new(app.directory_input.as_str())
        .style(if app.config_input_mode == ConfigInputMode::Directory {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        })
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
    f.render_widget(directory_input, form_chunks[current_chunk_index]);
    current_chunk_index += 1;

    if is_tv_show {
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

    // Year input for single files
    let show_single_year = (is_tv_show && app.files.len() == 1) || (app.file_type == FileType::Movie && app.files.len() == 1);
    if show_single_year {
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
    }

    // Multiple movie years input
    if is_multiple_movies {
        let movie_years_style = if app.config_input_mode == ConfigInputMode::MovieYears {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        
        let current_movie_name = if app.current_movie_index < app.files.len() {
            &app.files[app.current_movie_index].original_name
        } else {
            "Unknown"
        };
        
        let current_year = if app.current_movie_index < app.movie_years.len() {
            &app.movie_years[app.current_movie_index]
        } else {
            ""
        };
        
        let movie_years_title = format!("Movie {} of {} - Enter year for: {}", 
                                       app.current_movie_index + 1, 
                                       app.files.len(),
                                       current_movie_name);
        
        let year_display = if current_year.is_empty() {
            "[Enter year (optional)]".to_string()
        } else {
            current_year.to_string()
        };
        
        let movie_years_input = Paragraph::new(year_display.as_str())
            .style(movie_years_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(movie_years_title.as_str())
                    .border_style(if app.config_input_mode == ConfigInputMode::MovieYears {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Gray)
                    }),
            );
        f.render_widget(movie_years_input, form_chunks[current_chunk_index]);
        current_chunk_index += 1;
    }

    // IMDb choice (only for TV shows with multiple files)
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
    }

    // IMDb ID input (if needed and only for TV shows with multiple files)
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
    }

    // Instructions - Update to include navigation hints
    let instructions = match app.config_input_mode {
        ConfigInputMode::FileType => "Choose file type: T for TV Shows, M for Movies",
        ConfigInputMode::Directory => "Enter the directory path containing your video files (← Back)",
        ConfigInputMode::Season => {
            if app.season_input.is_empty() {
                "Season number is REQUIRED (e.g., S01, S1, 1, or 01) (← Back)"
            } else {
                "Season auto-detected! Press Enter to continue or type to edit (← Back)"
            }
        },
        ConfigInputMode::Year => {
            if app.file_type == FileType::TvShow && app.files.len() == 1 {
                "Year is REQUIRED for single TV episodes (e.g., 2023) (← Back)"
            } else {
                "Enter year or leave blank (press Enter to skip) (← Back)"
            }
        },
        ConfigInputMode::MovieYears => "Enter year for each movie (optional) (↑/↓ or ←/→ to navigate, ← Back)",
        ConfigInputMode::ImdbChoice => "Would you like to fetch episode titles from IMDb? (← Back)",
        ConfigInputMode::ImdbId => "Enter the IMDb series ID (found in the URL) (← Back)",
        ConfigInputMode::Confirm => "Review your settings and press Enter to continue (← Back)",
    };

    let help_lines = vec![
        Line::from(instructions),
        Line::from(""),
        Line::from("Navigation: ← Back | Enter: Next/Confirm | Esc: Quit"),
    ];

    let help_text = Paragraph::new(help_lines)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Instructions"));

    f.render_widget(help_text, chunks[2]);
}

pub fn render_main_screen(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
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

pub fn render_header(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
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

pub fn render_file_list(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
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
    if app.files.len() > area.height as usize - 2 {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));

        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut app.scroll_state.clone(),
        );
    }
}

pub fn render_status_bar(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
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

    f.render_widget(progress, chunks[0]);

    // Controls hint
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

pub fn render_preview_panel(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
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

pub fn render_help_popup(f: &mut Frame, _app: &App) {
    let popup_area = centered_rect(60, 50, f.area());

    let help_text = vec![
        Line::from(vec![
            Span::styled("Jellyfin Rename Tool - Help", Style::default().add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  Up/k    - Move up"),
        Line::from("  Down/j  - Move down"),
        Line::from(""),
        Line::from("Actions:"),
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
