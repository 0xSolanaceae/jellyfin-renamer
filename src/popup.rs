use eframe::egui;
use egui::{Color32, Stroke, Vec2, FontId, Align, Layout, RichText};
use crate::rename_engine::{RenameOperation, RenameResult};
use std::sync::{Arc, Mutex};

// Modern theme colors
struct ModernTheme {
    bg_main: Color32,
    bg_panel: Color32,
    bg_highlight: Color32,
    text_primary: Color32,
    text_secondary: Color32,
    accent: Color32,
    success: Color32,
    cancel: Color32,
    border: Color32,
    shadow: Color32,
}

impl Default for ModernTheme {
    fn default() -> Self {
        Self {
            bg_main: Color32::from_rgb(26, 32, 44),      // Dark background
            bg_panel: Color32::from_rgb(34, 41, 57),     // Panel background
            bg_highlight: Color32::from_rgb(45, 55, 72), // Highlight background
            text_primary: Color32::from_rgb(237, 242, 247), // Primary text
            text_secondary: Color32::from_rgb(160, 174, 192), // Secondary text
            accent: Color32::from_rgb(66, 153, 225),    // Accent blue
            success: Color32::from_rgb(72, 187, 120),   // Success green
            cancel: Color32::from_rgb(229, 62, 62),     // Cancel red
            border: Color32::from_rgb(74, 85, 104),     // Border color
            shadow: Color32::from_rgb(0, 0, 0).gamma_multiply(0.25),    // Shadow with alpha
        }
    }
}

pub struct RenameConfirmationApp {
    rename_op: RenameOperation,
    name_buffer: String,    // Buffer for editing the name
    theme: ModernTheme,
    confirmed: Option<bool>,
    animation_time: f32,
    prev_frame_time: f64,
}

impl RenameConfirmationApp {
    pub fn new(file_path: &str) -> Self {
        // Create a rename operation
        let rename_op = RenameOperation::new(file_path);
        
        // Initialize the edit buffer with the name without extension
        let name_buffer = rename_op.get_name_without_extension().to_string();
        
        Self {
            rename_op,
            name_buffer,
            theme: ModernTheme::default(),
            confirmed: None,
            animation_time: 0.0,
            prev_frame_time: 0.0,
        }
    }
    
    // Get the rename operation result when confirmed
    pub fn get_rename_result(&mut self) -> Option<RenameResult> {
        if self.confirmed == Some(true) {
            Some(self.rename_op.execute())
        } else {
            None
        }
    }
    
    // Get the new name when confirmed
    pub fn get_new_name(&self) -> Option<String> {
        if self.confirmed == Some(true) {
            Some(self.name_buffer.clone())
        } else {
            None
        }
    }
}

impl eframe::App for RenameConfirmationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update animation time
        let current_time = ctx.input(|i| i.time);
        let delta_time = if self.prev_frame_time > 0.0 {
            (current_time - self.prev_frame_time) as f32
        } else {
            0.0
        };
        self.prev_frame_time = current_time;
        self.animation_time += delta_time;
        
        // Handle key events
        ctx.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                self.confirmed = Some(false);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            
            if input.consume_key(egui::Modifiers::CTRL, egui::Key::Enter) {
                // Update rename_op with the current buffer name before confirming
                self.rename_op.update_new_name(self.name_buffer.clone());
                self.confirmed = Some(true);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        
        let theme = &self.theme;
        
        // Set up the visual style
        let visuals = egui::Visuals {
            window_fill: theme.bg_main,
            panel_fill: theme.bg_main,
            faint_bg_color: theme.bg_panel,
            widgets: egui::style::Widgets::dark(),
            window_shadow: egui::epaint::Shadow {
                offset: [0, 4],
                blur: 8,
                spread: 0,
                color: theme.shadow,
            },
            ..Default::default()
        };
        ctx.set_visuals(visuals);
        
        // Request a repaint for smooth animations
        ctx.request_repaint();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Set default text color
            ui.visuals_mut().override_text_color = Some(theme.text_primary);
            
            // Main container with some padding
            egui::Frame::new()
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        // Header with slight animation
                        let title_size = 28.0 + (self.animation_time.sin() * 0.5);
                        
                        // Logo and title with animation
                        ui.add_space(10.0);
                        let title = RichText::new("Jellyfin Rename")
                            .size(title_size)
                            .color(theme.accent)
                            .strong();
                        ui.heading(title);
                        
                        // Subtitle
                        ui.add_space(4.0);
                        ui.label(RichText::new("Rename your media files")
                            .color(theme.text_secondary)
                            .size(16.0));
                        
                        ui.add_space(24.0);
                        
                        // Card container for content
                        egui::Frame::new()
                            .fill(theme.bg_panel)
                            .corner_radius(12.0)
                            .stroke(Stroke::new(1.0, theme.border))
                            .shadow(egui::epaint::Shadow {
                                offset: [0, 2],
                                blur: 8,
                                spread: 0,
                                color: theme.shadow,
                            })
                            .inner_margin(24.0)
                            .outer_margin(8.0)
                            .show(ui, |ui| {
                                // File info layout
                                ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                                    // Original filename section
                                    ui.label(RichText::new("Original filename")
                                        .color(theme.text_secondary)
                                        .size(14.0));
                                    ui.add_space(4.0);
                                    
                                    // Original filename box
                                    egui::Frame::new()
                                        .fill(theme.bg_highlight)
                                        .corner_radius(8.0)
                                        .inner_margin(12.0)
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(self.rename_op.get_original_name())
                                                .color(theme.text_primary)
                                                .size(15.0)
                                                .monospace());
                                        });
                                    
                                    ui.add_space(16.0);
                                    
                                    // New filename section with editable text field
                                    ui.label(RichText::new("New filename")
                                        .color(theme.text_secondary)
                                        .size(14.0));
                                    ui.add_space(4.0);
                                    
                                    // Combine text field and extension display
                                    ui.horizontal(|ui| {
                                        // Create a text field with custom styling
                                        let text_edit = egui::TextEdit::singleline(&mut self.name_buffer)
                                            .font(FontId::monospace(15.0))
                                            .hint_text("Enter new filename")
                                            .desired_width(ui.available_width() - 60.0)
                                            .margin(egui::Vec2::new(10.0, 8.0));
                                        
                                        // Add the text field with custom styling
                                        let response = ui.add_sized(
                                            Vec2::new(ui.available_width() - 60.0, 36.0),
                                            text_edit.background_color(theme.bg_highlight)
                                        );
                                        if response.changed() {
                                            // Update the rename operation when text changes
                                            self.rename_op.update_new_name(self.name_buffer.clone());
                                        }
                                        
                                        // Auto-focus text field on start
                                        if self.animation_time < 0.5 {
                                            response.request_focus();
                                        }

                                        // Show extension as static text
                                        ui.label(RichText::new(self.rename_op.get_extension())
                                            .monospace()
                                            .size(15.0)
                                            .color(theme.text_secondary));
                                    });
                                    
                                    // Preview of full new name
                                    ui.add_space(8.0);                                    ui.label(
                                        RichText::new(format!("Preview: {}", self.rename_op.get_new_name()))
                                        .color(theme.text_secondary)
                                        .size(12.0)
                                    );
                                    
                                    ui.add_space(24.0);
                                    
                                    // Buttons with improved styling
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        // Confirm button with hover effect
                                        let success_button = egui::Button::new(
                                            RichText::new("Rename (Ctrl+Enter)")
                                                .color(Color32::WHITE)
                                                .size(14.0)
                                        )
                                        .min_size(Vec2::new(180.0, 36.0))
                                        .corner_radius(8.0)
                                        .fill(theme.success)
                                        .stroke(Stroke::NONE);
                                        
                                        if ui.add(success_button).clicked() {
                                            // Update the name before confirming
                                            self.rename_op.update_new_name(self.name_buffer.clone());
                                            self.confirmed = Some(true);
                                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                        }
                                        
                                        ui.add_space(12.0);
                                        
                                        // Cancel button
                                        let cancel_button = egui::Button::new(
                                            RichText::new("Cancel (Esc)")
                                                .color(theme.text_primary)
                                                .size(14.0)
                                        )
                                        .min_size(Vec2::new(120.0, 36.0))
                                        .corner_radius(8.0)
                                        .fill(theme.bg_highlight)
                                        .stroke(Stroke::new(1.0, theme.border));
                                        
                                        if ui.add(cancel_button).clicked() {
                                            self.confirmed = Some(false);
                                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                        }
                                    });
                                });
                            });
                        
                        // Footer with keyboard shortcuts
                        ui.add_space(16.0);
                        ui.label(RichText::new("Press Esc to cancel, Ctrl+Enter to confirm")
                            .size(12.0)
                            .color(theme.text_secondary));
                    });
                });
        });
    }
}

// Main function to show the dialog and perform rename if confirmed
pub fn show_rename_dialog(file_path: &str) -> bool {
    // Shared state for confirmation and new name
    let app_state = Arc::new(Mutex::new(None));
    let app_state_clone = app_state.clone();

    let app = RenameConfirmationAppWithCallback::new(file_path, Box::new(move |result, rename_op| {
        if result {
            // If confirmed, execute the rename operation
            match rename_op.execute() {
                RenameResult::Success(_) => {
                    *app_state_clone.lock().unwrap() = Some(true);
                },
                _ => {
                    // Failed to rename, treat as canceled
                    *app_state_clone.lock().unwrap() = Some(false);
                }
            }
        } else {
            // User canceled
            *app_state_clone.lock().unwrap() = Some(false);
        }
    }));

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.inner_size = Some(egui::vec2(500.0, 420.0)); // Slightly larger
    native_options.centered = true;
    native_options.vsync = true;

    // Run the app
    let _ = eframe::run_native(
        "Jellyfin Rename",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(app))
        }),
    );

    // Return the result of the operation
    app_state.lock().unwrap().unwrap_or(false)
}

// Wrapper app that takes a callback for confirmation and the rename operation
pub struct RenameConfirmationAppWithCallback {
    inner: RenameConfirmationApp,
    on_confirm: Box<dyn Fn(bool, &RenameOperation) + Send + Sync>,
}

impl RenameConfirmationAppWithCallback {
    pub fn new(file_path: &str, on_confirm: Box<dyn Fn(bool, &RenameOperation) + Send + Sync>) -> Self {
        Self {
            inner: RenameConfirmationApp::new(file_path),
            on_confirm,
        }
    }
}

impl eframe::App for RenameConfirmationAppWithCallback {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.inner.update(ctx, frame);
        if let Some(confirmed) = self.inner.confirmed {
            (self.on_confirm)(confirmed, &self.inner.rename_op);
            // Prevent repeated callback
            self.inner.confirmed = None;
        }
    }
}
