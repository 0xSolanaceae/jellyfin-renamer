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

#[derive(Debug, PartialEq)]
pub enum ConfigInputMode {
    FileType,
    Directory,
    Season,
    Year,
    MovieYears, // New mode for individual movie year input
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
    #[allow(dead_code)]
    pub original_name: String,
    pub new_name: String,
}
