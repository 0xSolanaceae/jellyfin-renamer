use std::path::{Path, PathBuf};
use std::fs;
use std::io;

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
