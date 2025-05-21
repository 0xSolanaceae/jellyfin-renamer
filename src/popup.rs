use eframe::egui;
use egui::{Color32, Stroke, Vec2};
use std::path::{Path, PathBuf};

// Gruvbox theme colors
struct GruvboxTheme {
    bg0: Color32,
    bg1: Color32,
    fg0: Color32,
    fg1: Color32,
    green: Color32,
    red: Color32,
    yellow: Color32,
    blue: Color32,
}

impl Default for GruvboxTheme {
    fn default() -> Self {
        Self {
            bg0: Color32::from_rgb(40, 40, 40),    // Background (dark)
            bg1: Color32::from_rgb(60, 56, 54),    // Lighter background
            fg0: Color32::from_rgb(251, 241, 199), // Foreground (light)
            fg1: Color32::from_rgb(235, 219, 178), // Dimmer foreground
            green: Color32::from_rgb(152, 151, 26),  // Green for Yes button
            red: Color32::from_rgb(204, 36, 29),     // Red for No button
            yellow: Color32::from_rgb(215, 153, 33), // Yellow accents
            blue: Color32::from_rgb(69, 133, 136),   // Blue accents
        }
    }
}

pub struct RenameConfirmationApp {
    #[allow(dead_code)]
    original_path: PathBuf,
    original_name: String,
    new_name: String,
    theme: GruvboxTheme,
    confirmed: Option<bool>,
}

impl RenameConfirmationApp {
    pub fn new(file_path: &str) -> Self {
        let path = Path::new(file_path);
        let original_name = path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
            
        let extension = path.extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_default();
            
        let new_name = format!("helloworld{}", extension);
        
        Self {
            original_path: path.to_path_buf(),
            original_name,
            new_name,
            theme: GruvboxTheme::default(),
            confirmed: None,
        }
    }
}

impl eframe::App for RenameConfirmationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = &self.theme;
        
        // Set up the visual style
        let visuals = egui::Visuals {
            window_fill: theme.bg0,
            panel_fill: theme.bg0,
            widgets: egui::style::Widgets::dark(),
            ..Default::default()
        };
        ctx.set_visuals(visuals);
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(theme.fg0);
            
            // Header
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                let title = egui::RichText::new("Jellyfin Rename")
                    .size(24.0)
                    .color(theme.yellow);
                ui.heading(title);
                ui.add_space(20.0);
            });
            
            // Content
            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Original filename:").color(theme.blue).size(16.0));
                    ui.add_space(4.0);
                    
                    // Original filename shown in a rounded box
                    let file_frame = egui::Frame::dark_canvas(ui.style())
                        .fill(theme.bg1)
                        .corner_radius(6.0)
                        .stroke(Stroke::new(1.0, theme.fg1))
                        .inner_margin(10.0);
                    
                    file_frame.show(ui, |ui| {
                        ui.label(egui::RichText::new(&self.original_name).color(theme.fg0).size(16.0));
                    });
                    
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("New filename:").color(theme.blue).size(16.0));
                    ui.add_space(4.0);
                    
                    // New filename shown in a rounded box
                    let new_file_frame = egui::Frame::dark_canvas(ui.style())
                        .fill(theme.bg1)
                        .corner_radius(6.0)
                        .stroke(Stroke::new(1.0, theme.fg1))
                        .inner_margin(10.0);
                    
                    new_file_frame.show(ui, |ui| {
                        ui.label(egui::RichText::new(&self.new_name).color(theme.fg0).size(16.0));
                    });
                    
                    ui.add_space(30.0);
                    
                    // Buttons
                    ui.horizontal(|ui| {
                        let button_size = Vec2::new(120.0, 40.0);
                        
                        // No button - styled with Gruvbox red
                        let no_button = egui::Button::new(
                            egui::RichText::new("Cancel").color(Color32::WHITE).size(16.0)
                        )
                        .fill(theme.red)
                        .min_size(button_size)
                        .corner_radius(6.0);
                        
                        if ui.add(no_button).clicked() {
                            self.confirmed = Some(false);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        
                        ui.add_space(20.0);
                        
                        // Yes button - styled with Gruvbox green
                        let yes_button = egui::Button::new(
                            egui::RichText::new("Rename").color(Color32::WHITE).size(16.0)
                        )
                        .fill(theme.green)
                        .min_size(button_size)
                        .corner_radius(6.0);
                        
                        if ui.add(yes_button).clicked() {
                            self.confirmed = Some(true);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
                ui.add_space(20.0);
            });
            
            ui.add_space(20.0);
        });
    }
}

pub fn show_rename_dialog(file_path: &str) -> bool {
    let app = RenameConfirmationApp::new(file_path);
    
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.inner_size = Some(egui::vec2(420.0, 360.0));
    
    match eframe::run_native(
        "Jellyfin Rename",
        native_options,
        Box::new(|cc| {
            // Set dark mode
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(app))
        }),
    ) {
        Ok(_) => true,
        Err(_) => false
    }
}