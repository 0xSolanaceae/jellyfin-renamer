use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use regex::Regex;
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct RenameConfig {
    pub directory: PathBuf,
    pub season: String,
    pub season_num: u32,
    pub year: Option<String>,
    pub use_imdb: bool,
    pub imdb_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileRename {
    pub original_path: PathBuf,
    pub original_name: String,
    pub new_name: String,
    pub episode_number: u32,
    pub episode_title: String,
}

#[derive(Debug, Clone)]
pub struct RenameResult {
    pub file_rename: FileRename,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug)]
pub struct RenameEngine {
    config: RenameConfig,
    imdb_titles: Vec<String>,
    standard_pattern: Regex,
    flexible_pattern: Regex,
}

impl RenameEngine {
    pub fn new(config: RenameConfig) -> Result<Self> {
        let standard_pattern = Regex::new(
            r"(?i)(?P<title>.*?)S(?P<season>\d{1,2})E(?P<episode>\d{2})(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi)$"
        )?;
        
        let flexible_pattern = Regex::new(
            r"(?i)(?P<title>.*?)\b(?P<season>\d{1,2})x(?P<episode>\d{2})\b(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi)$"
        )?;

        Ok(Self {
            config,
            imdb_titles: Vec::new(),
            standard_pattern,
            flexible_pattern,
        })
    }

    pub async fn fetch_imdb_titles(&mut self) -> Result<()> {
        if !self.config.use_imdb {
            return Ok(());
        }

        let imdb_id = self.config.imdb_id.as_ref()
            .ok_or_else(|| anyhow::anyhow!("IMDb ID is required when use_imdb is true"))?;

        println!("Fetching episode titles for {} from IMDb...", self.config.season);
        
        let titles = scrape_imdb_episodes(imdb_id, Some(self.config.season_num)).await?;
        
        if titles.is_empty() {
            println!("Could not fetch episode titles. Proceeding without IMDb titles.");
        } else {
            println!("Fetched {} episode titles.", titles.len());
            self.imdb_titles = titles;
        }

        Ok(())
    }

    pub fn scan_directory(&self) -> Result<Vec<FileRename>> {
        if !self.config.directory.exists() {
            return Err(anyhow::anyhow!("Directory does not exist: {:?}", self.config.directory));
        }

        let files: Vec<_> = fs::read_dir(&self.config.directory)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();

        let mut proposed_renames = Vec::new();
        let mut files_for_flexible = Vec::new();

        // Try standard pattern first
        for filename in &files {
            if let Some(rename) = self.process_file_standard(filename)? {
                if rename.original_name != rename.new_name {
                    proposed_renames.push(rename);
                }
            } else {
                files_for_flexible.push(filename.clone());
            }
        }

        // If no matches with standard pattern, try flexible pattern
        if proposed_renames.is_empty() && !files_for_flexible.is_empty() {
            println!("No files matched standard pattern, trying flexible pattern...");
            
            for filename in &files_for_flexible {
                if let Some(rename) = self.process_file_flexible(filename)? {
                    if rename.original_name != rename.new_name {
                        proposed_renames.push(rename);
                    }
                }
            }
        }

        Ok(proposed_renames)
    }

    pub fn process_file_standard(&self, filename: &str) -> Result<Option<FileRename>> {
        if let Some(captures) = self.standard_pattern.captures(filename) {
            let episode_number: u32 = captures.name("episode")
                .unwrap()
                .as_str()
                .parse()?;
            
            let season_number: u32 = captures.name("season")
                .unwrap()
                .as_str()
                .parse()?;
            
            let suffix = captures.name("suffix").unwrap().as_str();
            let extension = captures.name("extension").unwrap().as_str();

            let episode_title = if !self.imdb_titles.is_empty() && episode_number <= self.imdb_titles.len() as u32 {
                self.imdb_titles[(episode_number - 1) as usize].clone()
            } else {
                self.extract_episode_title_from_suffix(suffix)
            };

            let sanitized_title = sanitize_filename(&episode_title.replace(' ', "_"));
            let season_episode = format!("S{:02}E{:02}", season_number, episode_number);
            let new_name = format!("{}_({}).{}", sanitized_title, season_episode, extension);

            let original_path = self.config.directory.join(filename);
            
            return Ok(Some(FileRename {
                original_path,
                original_name: filename.to_string(),
                new_name,
                episode_number,
                episode_title,
            }));
        }

        Ok(None)
    }

    pub fn process_file_flexible(&self, filename: &str) -> Result<Option<FileRename>> {
        if let Some(captures) = self.flexible_pattern.captures(filename) {
            let episode_number: u32 = captures.name("episode")
                .unwrap()
                .as_str()
                .parse()?;
            
            let title = captures.name("title").unwrap().as_str();
            let extension = captures.name("extension").unwrap().as_str();

            let episode_title = if !self.imdb_titles.is_empty() && episode_number <= self.imdb_titles.len() as u32 {
                self.imdb_titles[(episode_number - 1) as usize].clone()
            } else {
                title.replace('.', "_")
            };

            let sanitized_title = sanitize_filename(&episode_title.replace(' ', "_"));
            let year_part = self.config.year.as_ref()
                .map(|y| format!("({})", y))
                .unwrap_or_default();
            
            let new_name = format!("{}_{}{}.{}", 
                sanitized_title, 
                self.config.season, 
                year_part, 
                extension
            );

            let original_path = self.config.directory.join(filename);
            
            return Ok(Some(FileRename {
                original_path,
                original_name: filename.to_string(),
                new_name,
                episode_number,
                episode_title,
            }));
        }

        Ok(None)
    }

    fn extract_episode_title_from_suffix(&self, suffix: &str) -> String {
        let title_parts: Vec<&str> = suffix.split('.').collect();
        let mut meaningful_parts = Vec::new();
        
        for part in title_parts {
            let part_lower = part.to_lowercase();
            if part_lower.contains("1080p") || 
               part_lower.contains("720p") || 
               part_lower.contains("bluray") || 
               part_lower.contains("x264") || 
               part_lower.contains("x265") || 
               part_lower.contains("web-dl") || 
               part_lower.contains("webrip") {
                break;
            }
            if !part.is_empty() {
                meaningful_parts.push(part);
            }
        }
        
        meaningful_parts.join(" ")
    }

    pub async fn rename_file(&self, file_rename: &FileRename) -> RenameResult {
        let new_path = self.config.directory.join(&file_rename.new_name);
        
        match fs::rename(&file_rename.original_path, &new_path) {
            Ok(_) => RenameResult {
                file_rename: file_rename.clone(),
                success: true,
                error_message: None,
            },
            Err(e) => RenameResult {
                file_rename: file_rename.clone(),
                success: false,
                error_message: Some(e.to_string()),
            }
        }
    }

    pub async fn rename_files(&self, files: &[FileRename]) -> Vec<RenameResult> {
        let mut results = Vec::new();
        
        for file in files {
            let result = self.rename_file(file).await;
            results.push(result);
        }
        
        results
    }
}

// Helper function to sanitize filenames
pub fn sanitize_filename(filename: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();
    re.replace_all(filename, "_").to_string()
}

// Helper function to extract season number from directory name
pub fn extract_season_from_directory(dir_name: &str) -> Option<u32> {
    // Try multiple patterns for season detection
    let patterns = [
        r"s(?:eason\s*)?(\d+)",           // s1, season 1, s01, season 01
        r"(?:season\s+)(\d+)",            // season 1, season 01
        r"(\d+)(?:st|nd|rd|th)\s*season", // 1st season, 2nd season
        r"series\s*(\d+)",                // series 1, series 01
    ];
    
    let dir_lower = dir_name.to_lowercase();
    
    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(captures) = re.captures(&dir_lower) {
                if let Some(season_match) = captures.get(1) {
                    if let Ok(season_num) = season_match.as_str().parse::<u32>() {
                        return Some(season_num);
                    }
                }
            }
        }
    }
    
    None
}

// Helper function to extract season number from file name
pub fn extract_season_from_filename(filename: &str) -> Option<u32> {
    // Try to extract season from standard patterns in filename
    let patterns = [
        r"S(\d{1,2})E\d{2}",              // S01E01, S1E01
        r"(?:season\s*)?(\d+)x\d{2}",     // 1x01, season 1x01
        r"s(\d+)e\d+",                    // s1e01, s01e01
    ];
    
    let filename_lower = filename.to_lowercase();
    
    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(captures) = re.captures(&filename_lower) {
                if let Some(season_match) = captures.get(1) {
                    if let Ok(season_num) = season_match.as_str().parse::<u32>() {
                        return Some(season_num);
                    }
                }
            }
        }
    }
    
    None
}

// IMDb scraping functionality
pub async fn scrape_imdb_episodes(imdb_id: &str, season: Option<u32>) -> Result<Vec<String>> {
    let mut url = format!("https://www.imdb.com/title/{}/episodes", imdb_id);
    if let Some(season_num) = season {
        url.push_str(&format!("?season={}", season_num));
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .context("Failed to fetch IMDb page")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    
    // Try multiple selectors as IMDb's structure can vary
    let selectors = [
        "div.ipc-title.ipc-title--base.ipc-title--title .ipc-title__text",
        ".titleColumn a",
        ".ipc-title__text",
        "h3.ipc-title__text",
    ];

    let mut results = Vec::new();
    
    for selector_str in &selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<String>();
                if text.contains('∙') {
                    if let Some(title) = text.split('∙').last() {
                        let cleaned_title = title.trim().to_string();
                        if !cleaned_title.is_empty() {
                            results.push(cleaned_title);
                        }
                    }
                } else if !text.trim().is_empty() && !text.contains("S.") {
                    // Filter out episode numbers like "S1.E1"
                    results.push(text.trim().to_string());
                }
            }
        }
        
        if !results.is_empty() {
            break;
        }
    }

    // Remove duplicates while preserving order
    let mut unique_results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    for result in results {
        if seen.insert(result.clone()) {
            unique_results.push(result);
        }
    }

    Ok(unique_results)
}

// Interactive configuration builder
pub struct ConfigBuilder {
    directory: Option<PathBuf>,
    season: Option<String>,
    season_num: Option<u32>,
    year: Option<String>,
    use_imdb: bool,
    imdb_id: Option<String>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            directory: None,
            season: None,
            season_num: None,
            year: None,
            use_imdb: false,
            imdb_id: None,
        }
    }

    pub fn directory<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.directory = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn season(mut self, season: String) -> Self {
        // Extract season number from season string
        if let Some(season_num) = season.strip_prefix('S').or_else(|| season.strip_prefix('s')) {
            if let Ok(num) = season_num.parse::<u32>() {
                self.season_num = Some(num);
                self.season = Some(format!("S{:02}", num));
            }
        } else if let Ok(num) = season.parse::<u32>() {
            self.season_num = Some(num);
            self.season = Some(format!("S{:02}", num));
        }
        self
    }

    pub fn year(mut self, year: Option<String>) -> Self {
        self.year = year;
        self
    }

    pub fn imdb(mut self, imdb_id: Option<String>) -> Self {
        self.use_imdb = imdb_id.is_some();
        self.imdb_id = imdb_id;
        self
    }

    pub fn build(self) -> Result<RenameConfig> {
        let directory = self.directory
            .ok_or_else(|| anyhow::anyhow!("Directory is required"))?;
        
        let season = self.season
            .ok_or_else(|| anyhow::anyhow!("Season is required"))?;
            
        let season_num = self.season_num
            .ok_or_else(|| anyhow::anyhow!("Season number is required"))?;

        Ok(RenameConfig {
            directory,
            season,
            season_num,
            year: self.year,
            use_imdb: self.use_imdb,
            imdb_id: self.imdb_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Test: File/Name"), "Test_ File_Name");
        assert_eq!(sanitize_filename("Normal_File.Name"), "Normal_File.Name");
    }

    #[test]
    fn test_extract_season_from_directory() {
        assert_eq!(extract_season_from_directory("Show.S01"), Some(1));
        assert_eq!(extract_season_from_directory("Show.s02.1080p"), Some(2));
        assert_eq!(extract_season_from_directory("Random.Folder"), None);
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = ConfigBuilder::new()
            .directory("/test/path")
            .season("S01".to_string())
            .year(Some("2023".to_string()))
            .build()
            .unwrap();

        assert_eq!(config.season, "S01");
        assert_eq!(config.season_num, 1);
        assert_eq!(config.year, Some("2023".to_string()));
    }
}