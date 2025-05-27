use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use regex::Regex;
use reqwest;
use scraper::{Html, Selector};

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    TvShow,
    Movie,
}

#[derive(Debug, Clone)]
pub struct RenameConfig {
    pub directory: PathBuf,
    pub season: String,
    pub season_num: u32,
    pub year: Option<String>,
    pub use_imdb: bool,
    pub imdb_id: Option<String>,
    pub file_type: FileType,
}

#[derive(Debug, Clone)]
pub struct FileRename {
    pub original_path: PathBuf,
    pub original_name: String,
    pub new_name: String,
    pub episode_number: u32,
    pub season_number: u32,
    pub episode_title: String,
    pub needs_rename: bool, // Whether this file actually needs to be renamed
}

#[derive(Debug, Clone)]
pub struct RenameResult {
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug)]
pub struct RenameEngine {
    pub config: RenameConfig,
    imdb_titles: Vec<String>,
    standard_pattern: Regex,
    flexible_pattern: Regex,
    movie_pattern: Regex,
}

impl RenameEngine {
    pub fn new(config: RenameConfig) -> Result<Self> {
        let standard_pattern = Regex::new(
            r"(?i)(?P<title>.*?)S(?P<season>\d{1,2})E(?P<episode>\d{2})(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi)$"
        )?;
        
        let flexible_pattern = Regex::new(
            r"(?i)(?P<title>.*?)\b(?P<season>\d{1,2})x(?P<episode>\d{2})\b(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi)$"
        )?;

        // Movie pattern to handle prefixes like "Watch", suffixes like torrent site names
        let movie_pattern = Regex::new(
            r"(?i)^(?:Watch\s+)?(?P<title>.*?)(?:\s*-\s*(?P<suffix>.*?))?\.(?P<extension>mkv|mp4|avi)$"
        )?;

        Ok(Self {
            imdb_titles: Vec::new(),
            standard_pattern,
            flexible_pattern,
            movie_pattern,
            config,
        })
    }    pub async fn fetch_imdb_titles(&mut self) -> Result<()> {
        if !self.config.use_imdb {
            return Ok(());
        }

        let imdb_id = self.config.imdb_id.as_ref()
            .ok_or_else(|| anyhow::anyhow!("IMDb ID is required when use_imdb is true"))?;

        let titles = scrape_imdb_episodes(imdb_id, Some(self.config.season_num)).await?;
        
        if !titles.is_empty() {
            self.imdb_titles = titles;
        }

        Ok(())
    }

    pub fn get_imdb_titles(&self) -> &Vec<String> {
        &self.imdb_titles
    }    pub fn scan_directory(&self) -> Result<Vec<FileRename>> {
        if !self.config.directory.exists() {
            return Err(anyhow::anyhow!("Directory does not exist: {:?}", self.config.directory));
        }

        let files: Vec<_> = fs::read_dir(&self.config.directory)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();

        let mut proposed_renames = Vec::new();
        
        match self.config.file_type {
            FileType::TvShow => {
                // Process as TV show episodes
                let mut files_for_flexible = Vec::new();
                
                // Try standard pattern first
                for filename in &files {
                    if let Some(rename) = self.process_file_standard(filename)? {
                        proposed_renames.push(rename);
                    } else {
                        files_for_flexible.push(filename.clone());
                    }
                }

                // If no matches with standard pattern, try flexible pattern
                if proposed_renames.is_empty() && !files_for_flexible.is_empty() {
                    println!("No files matched standard pattern, trying flexible pattern...");
                    
                    for filename in &files_for_flexible {
                        if let Some(rename) = self.process_file_flexible(filename)? {
                            proposed_renames.push(rename);
                        }
                    }
                }
            },
            FileType::Movie => {
                // Process as movies
                for filename in &files {
                    if let Some(rename) = self.process_file_movie(filename)? {
                        proposed_renames.push(rename);
                    }
                }
            }
        }

        Ok(proposed_renames)
    }pub fn process_file_standard(&self, filename: &str) -> Result<Option<FileRename>> {
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
            let needs_rename = filename != &new_name;
            
            return Ok(Some(FileRename {
                original_path,
                original_name: filename.to_string(),
                new_name,
                episode_number,
                season_number,
                episode_title,
                needs_rename,
            }));
        }

        Ok(None)
    }pub fn process_file_flexible(&self, filename: &str) -> Result<Option<FileRename>> {
        if let Some(captures) = self.flexible_pattern.captures(filename) {
            let episode_number: u32 = captures.name("episode")
                .unwrap()
                .as_str()
                .parse()?;
            
            let season_number: u32 = captures.name("season")
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
            let needs_rename = filename != &new_name;
            
            return Ok(Some(FileRename {
                original_path,
                original_name: filename.to_string(),
                new_name,
                episode_number,
                season_number,
                episode_title,
                needs_rename,
            }));
        }Ok(None)
    }

    // Process a file with a manual season override
    pub fn process_file_with_manual_season(&self, filename: &str, manual_season: u32) -> Result<Option<FileRename>> {
        // Try to extract episode information first using standard or flexible pattern
        let mut file_rename_result = self.process_file_standard(filename)?;
        if file_rename_result.is_none() {
            file_rename_result = self.process_file_flexible(filename)?;
        }
        
        // If neither standard nor flexible patterns work, try movie pattern
        if file_rename_result.is_none() {
            file_rename_result = self.process_file_movie(filename)?;
        }
        
        if let Some(mut file_rename) = file_rename_result {
            // For episodes (episode_number > 0), override the season
            // For movies (episode_number = 0), keep the movie formatting
            if file_rename.episode_number > 0 {
                // This is a TV episode - override the season
                let extension = std::path::Path::new(filename)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("mkv");
                    
                let sanitized_title = sanitize_filename(&file_rename.episode_title.replace(' ', "_"));
                
                // Create new filename with manual season
                let season_episode = format!("S{:02}E{:02}", manual_season, file_rename.episode_number);
                
                let new_name = if let Some(year) = &self.config.year {
                    format!("{}_({}({}).{}", sanitized_title, season_episode, year, extension)
                } else {
                    format!("{}_({}).{}", sanitized_title, season_episode, extension)
                };
                
                // Update file rename with new name and check if rename is needed
                file_rename.new_name = new_name;
                file_rename.needs_rename = filename != &file_rename.new_name;
                file_rename.season_number = manual_season;
            }
            // For movies (episode_number = 0), the formatting is already correct from process_file_movie
            
            return Ok(Some(file_rename));
        }
        
        // No matching pattern found
        Ok(None)
    }

    // Process a movie file (no season/episode, just clean title with year)
    pub fn process_file_movie(&self, filename: &str) -> Result<Option<FileRename>> {
        if let Some(captures) = self.movie_pattern.captures(filename) {
            let raw_title = captures.name("title").unwrap().as_str();
            let extension = captures.name("extension").unwrap().as_str();
            let suffix = captures.name("suffix").map(|s| s.as_str()).unwrap_or("");
            
            // Clean the title by removing common prefixes, suffixes, and quality indicators
            let cleaned_title = self.clean_movie_title(raw_title, suffix);
            
            if cleaned_title.is_empty() {
                return Ok(None);
            }
            
            let sanitized_title = sanitize_filename(&cleaned_title.replace(' ', "_"));
              // For movies, format as: Title_(year).extension with underscore before year
            let year_part = self.config.year.as_ref()
                .map(|y| format!("_({})", y))
                .unwrap_or_default();
                  let new_name = format!("{}{}.{}", sanitized_title, year_part, extension);
            
            let file_rename = FileRename {
                original_path: self.config.directory.join(filename),
                original_name: filename.to_string(),
                new_name: new_name.clone(),
                episode_title: cleaned_title,
                episode_number: 0, // No episode number for movies
                season_number: 1, // Default season for movies
                needs_rename: filename != new_name,
            };
            
            return Ok(Some(file_rename));
        }
        
        Ok(None)
    }

    // Clean movie title by removing prefixes, suffixes, and quality indicators
    fn clean_movie_title(&self, title: &str, suffix: &str) -> String {
        let mut cleaned = title.trim().to_string();
        
        // Remove common prefixes (case insensitive)
        let prefixes = ["watch", "download", "stream"];
        for prefix in &prefixes {
            let pattern = format!("^{}", regex::escape(prefix));
            if let Ok(re) = Regex::new(&format!("(?i){}", pattern)) {
                cleaned = re.replace(&cleaned, "").trim().to_string();
            }
        }
        
        // Remove common suffixes and quality indicators from both title and suffix
        let quality_indicators = [
            "1080p", "720p", "480p", "4k", "hd", "bluray", "blu-ray", "dvdrip", 
            "webrip", "web-dl", "hdtv", "x264", "x265", "h264", "h265", "xvid",
            "aac", "ac3", "dts", "5.1", "7.1", "atmos", "hdr", "dolby",
            "yify", "rarbg", "ettv", "eztv", "torrent", "bit", "hexa watch"
        ];
        
        // Process suffix for additional title information, but filter out quality indicators
        if !suffix.is_empty() {
            let suffix_words: Vec<&str> = suffix.split(&[' ', '.', '-', '_'][..])
                .filter(|word| !word.is_empty())
                .filter(|word| {
                    let word_lower = word.to_lowercase();
                    !quality_indicators.iter().any(|indicator| word_lower.contains(indicator))
                })
                .collect();
            
            // If suffix has meaningful content (not just quality indicators), it might be part of title
            if !suffix_words.is_empty() && suffix_words.len() <= 3 {
                // Only add if it looks like part of the title (short phrases)
                let suffix_text = suffix_words.join(" ");
                if !cleaned.to_lowercase().contains(&suffix_text.to_lowercase()) {
                    cleaned = format!("{} {}", cleaned, suffix_text);
                }
            }
        }
          // Final cleanup: remove quality indicators from the main title
        for indicator in &quality_indicators {
            let pattern = format!(r"(?i)\b{}\b", regex::escape(indicator));
            if let Ok(re) = Regex::new(&pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }
          // Extract and remove years from title (19xx or 20xx), but only if no year is already configured
        if self.config.year.is_none() {
            if let Ok(year_regex) = Regex::new(r"\b(19\d{2}|20\d{2})\b") {
                if let Some(_year_match) = year_regex.find(&cleaned) {
                    // Remove the year from the title
                    cleaned = year_regex.replace(&cleaned, "").trim().to_string();
                    // Note: In a real implementation, you might want to store the extracted year
                    // somewhere to use it in the filename format
                }
            }
        }
        
        // Clean up extra whitespace and punctuation
        cleaned = cleaned.trim()
            .replace("  ", " ")
            .replace(" .", "")
            .replace(" -", "")
            .replace("- ", "")
            .trim_end_matches('-')
            .trim_end_matches('.')
            .trim()
            .to_string();
        
        // Capitalize each word properly
        cleaned.split_whitespace()
            .map(|word| {
                let mut chars: Vec<char> = word.chars().collect();
                if !chars.is_empty() {
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                }
                chars.into_iter().collect()
            })
            .collect::<Vec<String>>()
            .join(" ")
    }    // Extract episode title from filename suffix
    fn extract_episode_title_from_suffix(&self, suffix: &str) -> String {
        // Clean the suffix by removing common quality indicators and torrent site names
        let cleaned = suffix.trim().to_string();
        
        // Remove quality indicators and technical info
        let quality_indicators = [
            "1080p", "720p", "480p", "4k", "2160p", "hd", "fhd", "uhd",
            "x264", "x265", "h264", "h265", "xvid", "divx", "mpeg",
            "bluray", "blu-ray", "webrip", "web-dl", "hdtv", "dvdrip", "brrip",
            "aac", "ac3", "mp3", "dts", "flac", "dd5.1", "dd+", "atmos",
            "5.1", "7.1", "2.0", "stereo", "mono",
            "pahe.in", "rarbg", "yify", "ettv", "eztv", "torrent", "bit",
            "hexa", "watch", "download", "stream", "720p.bluray", "1080p.bluray"
        ];
        
        // Split by common separators and filter out quality indicators
        let words: Vec<&str> = cleaned.split(&['.', '-', '_', ' '][..])
            .filter(|word| !word.is_empty())
            .filter(|word| {
                let word_lower = word.to_lowercase();
                // Keep words that are not quality indicators
                !quality_indicators.iter().any(|indicator| {
                    word_lower == indicator.to_lowercase() || 
                    word_lower.contains(&indicator.to_lowercase())
                })
            })
            .collect();
        
        // Take the first meaningful word(s) as the episode title
        let mut title_words = Vec::new();
        for word in words {
            // Stop if we hit common technical terms that usually come after title
            if word.to_lowercase().contains("x264") || 
               word.to_lowercase().contains("x265") ||
               word.to_lowercase().contains("bluray") ||
               word.to_lowercase().contains("1080p") ||
               word.to_lowercase().contains("720p") ||
               word.len() < 2 {
                break;
            }
            title_words.push(word);
            
            // Typically episode titles are 1-3 words
            if title_words.len() >= 3 {
                break;
            }
        }
        
        if title_words.is_empty() {
            return "Episode".to_string();
        }
        
        // Join the title words and capitalize properly
        let title = title_words.join(" ");
        
        // Capitalize each word properly
        title.split_whitespace()
            .map(|word| {
                let mut chars: Vec<char> = word.chars().collect();
                if !chars.is_empty() {
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                }
                chars.into_iter().collect()
            })
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub async fn rename_file(&self, file_rename: &FileRename) -> RenameResult {
        let new_path = self.config.directory.join(&file_rename.new_name);
        
        match fs::rename(&file_rename.original_path, &new_path) {
            Ok(_) => RenameResult {
                success: true,
                error_message: None,
            },
            Err(e) => RenameResult {
                success: false,
                error_message: Some(e.to_string()),
            }
        }    }

    // Process a file with a specific year (for multiple movies with individual years)
    pub fn process_file_with_year(&self, filename: &str, year: Option<String>) -> Result<Option<FileRename>> {
        // Create a temporary config with the specific year
        let mut temp_config = self.config.clone();
        temp_config.year = year;
        
        // Create a temporary engine with the updated config
        let temp_engine = RenameEngine {
            config: temp_config,
            imdb_titles: self.imdb_titles.clone(),
            standard_pattern: self.standard_pattern.clone(),
            flexible_pattern: self.flexible_pattern.clone(),
            movie_pattern: self.movie_pattern.clone(),
        };
        
        // Try different processing methods based on file type
        match self.config.file_type {
            FileType::TvShow => {
                // For TV shows, try standard then flexible patterns
                if let Some(file_rename) = temp_engine.process_file_standard(filename)? {
                    return Ok(Some(file_rename));
                } else if let Some(file_rename) = temp_engine.process_file_flexible(filename)? {
                    return Ok(Some(file_rename));
                }
            },
            FileType::Movie => {
                // For movies, use movie processing
                if let Some(file_rename) = temp_engine.process_file_movie(filename)? {
                    return Ok(Some(file_rename));
                }
            }
        }
        
        Ok(None)
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
    file_type: Option<FileType>,
}

impl ConfigBuilder {    pub fn new() -> Self {
        Self {
            directory: None,
            season: None,
            season_num: None,
            year: None,
            use_imdb: false,
            imdb_id: None,
            file_type: None,
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
    }    pub fn imdb(mut self, imdb_id: Option<String>) -> Self {
        self.use_imdb = imdb_id.is_some();
        self.imdb_id = imdb_id;
        self
    }

    pub fn file_type(mut self, file_type: FileType) -> Self {
        self.file_type = Some(file_type);
        self
    }    pub fn build(self) -> Result<RenameConfig> {
        let directory = self.directory
            .ok_or_else(|| anyhow::anyhow!("Directory is required"))?;
        
        let file_type = self.file_type
            .ok_or_else(|| anyhow::anyhow!("File type is required"))?;
        
        // For TV shows, season is required
        let (season, season_num) = if file_type == FileType::TvShow {
            let season = self.season
                .ok_or_else(|| anyhow::anyhow!("Season is required for TV shows"))?;
            let season_num = self.season_num
                .ok_or_else(|| anyhow::anyhow!("Season number is required for TV shows"))?;
            (season, season_num)
        } else {
            // For movies, use defaults
            (String::from("S01"), 1)
        };

        Ok(RenameConfig {
            directory,
            season,
            season_num,
            year: self.year,
            use_imdb: self.use_imdb,
            imdb_id: self.imdb_id,
            file_type,
        })
    }
}