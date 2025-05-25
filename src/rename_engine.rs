use std::path::{Path, PathBuf};
use std::fs;
use std::io;

/// Processes a filename to remove unwanted patterns and improve readability
pub fn process_filename(filename: &str) -> String {
    println!("Debug: Starting with filename: '{}'", filename);
    
    let mut processed = filename.to_string();
    
    // Remove common unwanted patterns (case insensitive)
    let patterns_to_remove = [
        // Torrent sites and groups
        "www.yts.mx", "YTS.MX", "yts.mx", "[YTS.MX]", "(YTS.MX)",
        "YIFY", "yify", "[YIFY]", "(YIFY)",
        "RARBG", "rarbg", "[RARBG]", "(RARBG)",
        "1337x", "[1337x]", "(1337x)",
        "TGx", "[TGx]", "(TGx)",
        
        // Video quality indicators
        "1080p", "720p", "480p", "2160p", "4K", "HD", "HDTV",
        "BluRay", "Blu-Ray", "BrRip", "BDRip", "DVDRip", "WEBRip", "WEB-DL",
        "CAMRip", "TS", "TC", "SCREENER", "R5",
        
        // Video codecs
        "x264", "x265", "H.264", "H.265", "HEVC", "AVC", "XviD", "DivX",
        "VP9", "AV1",
        
        // Audio codecs
        "AAC", "AC3", "DTS", "MP3", "FLAC", "Atmos", "DTS-HD",
        
        // Release info
        "EXTENDED", "UNRATED", "DIRECTORS.CUT", "REMASTERED",
        "LIMITED", "INTERNAL", "PROPER", "REPACK",
    ];
    
    // Case insensitive pattern removal
    for pattern in &patterns_to_remove {
        let pattern_lower = pattern.to_lowercase();
        let original_len = processed.len();
        
        // Keep replacing until no more matches found
        loop {
            let processed_lower = processed.to_lowercase();
            if let Some(pos) = processed_lower.find(&pattern_lower) {
                processed = format!("{}{}", &processed[..pos], &processed[pos + pattern.len()..]);
            } else {
                break;
            }
        }
        
        if processed.len() != original_len {
            println!("Debug: Removed '{}', now: '{}'", pattern, processed);
        }
    }
    
    // Remove brackets and their contents if they seem to be release info
    let bracket_patterns = [
        (r"\[.*?\]", "square brackets"),
        (r"\(.*?\)", "parentheses"),
    ];
    
    for (_, desc) in &bracket_patterns {
        let original = processed.clone();
        // Simple bracket removal - replace with space
        processed = processed.chars().collect::<Vec<char>>()
            .iter()
            .fold((String::new(), 0, false), |(mut result, depth, in_brackets), &ch| {
                match ch {
                    '[' | '(' => {
                        if depth == 0 { result.push(' '); }
                        (result, depth + 1, true)
                    },
                    ']' | ')' => {
                        let new_depth = if depth > 0 { depth - 1 } else { 0 };
                        if new_depth == 0 { result.push(' '); }
                        (result, new_depth, new_depth > 0)
                    },
                    _ => {
                        if !in_brackets {
                            result.push(ch);
                        }
                        (result, depth, in_brackets)
                    }
                }
            }).0;
            
        if processed != original {
            println!("Debug: Removed {}, now: '{}'", desc, processed);
        }
    }
    
    // Replace common separators with spaces
    let separators = [".", "_", "-", "+"];
    for sep in &separators {
        if processed.contains(sep) {
            processed = processed.replace(sep, " ");
            println!("Debug: Replaced '{}' with spaces, now: '{}'", sep, processed);
        }
    }
    
    // Clean up multiple spaces and trim
    while processed.contains("  ") {
        processed = processed.replace("  ", " ");
    }
    
    processed = processed.trim().to_string();
    
    // Remove leading/trailing dots, spaces, and other punctuation
    processed = processed.trim_matches(&['.', ' ', '-', '_', '(', ')', '[', ']', ','] as &[char]).to_string();
    
    // Capitalize first letter of each word for better presentation
    let words: Vec<String> = processed
        .split_whitespace()
        .map(|word| {
            let mut chars: Vec<char> = word.chars().collect();
            if !chars.is_empty() {
                chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
            }
            chars.into_iter().collect()
        })
        .collect();
    
    processed = words.join(" ");
    
    println!("Debug: Final result: '{}'", processed);
    
    processed
}

/// Represents the result of a rename operation
#[derive(Debug)]
pub enum RenameResult {
    Success(PathBuf),    // Success with the new path
    AlreadyExists,       // Target file already exists
    NoPermission,        // No permission to rename
    SourceNotFound,      // Source file not found
    OtherError(String),  // Other error with message
}

/// Represents a file to be renamed
pub struct RenameOperation {
    original_path: PathBuf,
    new_name: String,
    original_name: String,
    extension: String,
    name_without_extension: String,
}

impl RenameOperation {
    /// Create a new rename operation from a file path
    pub fn new(file_path: &str) -> Self {
        let path = Path::new(file_path);
        let original_name = path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
            
        let extension = path.extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_default();
            
        let name_without_extension = path.file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
            
        // Default new name is the same as original
        let new_name = original_name.clone();
        
        Self {
            original_path: path.to_path_buf(),
            original_name,
            new_name,
            extension,
            name_without_extension,
        }
    }
    
    /// Update the new name (without extension)
    pub fn update_new_name(&mut self, name: String) {
        self.name_without_extension = name;
        self.new_name = format!("{}{}", self.name_without_extension, self.extension);
    }
    
    /// Get the original name with extension
    pub fn get_original_name(&self) -> &str {
        &self.original_name
    }
    
    /// Get the new name with extension
    pub fn get_new_name(&self) -> &str {
        &self.new_name
    }
    
    /// Get the extension (with dot)
    pub fn get_extension(&self) -> &str {
        &self.extension
    }
    
    /// Get the name without extension that can be edited
    pub fn get_name_without_extension(&self) -> &str {
        &self.name_without_extension
    }
    
    /// Get the original path
    pub fn get_original_path(&self) -> &PathBuf {
        &self.original_path
    }
    
    /// Calculate the new full path
    pub fn get_new_path(&self) -> PathBuf {
        let parent = self.original_path.parent().unwrap_or(Path::new(""));
        parent.join(&self.new_name)
    }
    
    /// Execute the rename operation
    pub fn execute(&self) -> RenameResult {
        if self.original_name == self.new_name {
            return RenameResult::Success(self.original_path.clone());
        }
        
        let new_path = self.get_new_path();
        
        if !self.original_path.exists() {
            return RenameResult::SourceNotFound;
        }
        
        if new_path.exists() {
            return RenameResult::AlreadyExists;
        }
        
        match fs::rename(&self.original_path, &new_path) {
            Ok(_) => RenameResult::Success(new_path),
            Err(e) => match e.kind() {
                io::ErrorKind::PermissionDenied => RenameResult::NoPermission,
                io::ErrorKind::NotFound => RenameResult::SourceNotFound,
                _ => RenameResult::OtherError(e.to_string()),
            }
        }    }
}

/// Rename a single file by processing its filename
pub fn rename_file(file_path: &str) -> bool {
    println!("File path: {}", file_path);
    
    let path = Path::new(file_path);
    
    if !path.exists() {
        println!("✗ Error: File '{}' not found!", file_path);
        return false;
    }

    if !path.is_file() {
        println!("✗ Error: '{}' is not a file!", file_path);
        return false;
    }

    let mut rename_op = RenameOperation::new(file_path);
    
    // Get the original filename without extension
    let original_name = rename_op.get_name_without_extension();
    println!("Original name: {}", original_name);
    
    // Generate new name by processing the original name
    let new_name = process_filename(original_name);
    println!("Processed name: {}", new_name);
    
    // If the name doesn't change, skip renaming
    if new_name == original_name {
        println!("ℹ No changes needed for: {}", rename_op.get_original_name());
        return true; // Consider this a success since no action was needed
    }
    
    // Update the rename operation with the new name
    rename_op.update_new_name(new_name);
    
    println!("Renaming: {} -> {}", rename_op.get_original_name(), rename_op.get_new_name());
    
    // Execute the rename
    match rename_op.execute() {
        RenameResult::Success(_) => {
            println!("✓ Successfully renamed to: {}", rename_op.get_new_name());
            true
        }
        RenameResult::AlreadyExists => {
            println!("✗ Error: Target file '{}' already exists!", rename_op.get_new_name());
            false
        }
        RenameResult::NoPermission => {
            println!("✗ Error: No permission to rename '{}'!", file_path);
            false
        }
        RenameResult::SourceNotFound => {
            println!("✗ Error: Source file '{}' not found!", file_path);
            false
        }
        RenameResult::OtherError(msg) => {
            println!("✗ Error renaming '{}': {}", file_path, msg);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rename_operation_creation() {
        let op = RenameOperation::new("/path/to/file.txt");
        assert_eq!(op.get_original_name(), "file.txt");
        assert_eq!(op.get_extension(), ".txt");
        assert_eq!(op.get_name_without_extension(), "file");
    }
    
    #[test]
    fn test_update_new_name() {
        let mut op = RenameOperation::new("/path/to/file.txt");
        op.update_new_name("newfile".to_string());
        assert_eq!(op.get_new_name(), "newfile.txt");
    }
}
