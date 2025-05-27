// TUI module - Re-exports from organized submodules
pub mod app;
pub mod events;
pub mod models;
pub mod rendering;
pub mod utils;

// Re-export the main entry points and types for backward compatibility
pub use events::run_tui;