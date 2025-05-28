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
    pub needs_rename: bool,
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
            r"(?i)(?P<title>.*?)S(?P<season>\d{1,2})E(?P<episode>\d{2})(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi|ts)$"
        )?;
          let flexible_pattern = Regex::new(
            r"(?i)(?P<title>.*?)\b(?P<season>\d{1,2})x(?P<episode>\d{2})\b(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi|ts)$"
        )?;

        let movie_pattern = Regex::new(
            r"(?i)^(?:Watch\s+)?(?P<title>.*?)(?:\s*-\s*(?P<suffix>.*?))?\.(?P<extension>mkv|mp4|avi|ts)$"
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
    
    #[allow(dead_code)]
    pub fn get_imdb_titles(&self) -> &Vec<String> {
        &self.imdb_titles
    }    pub fn scan_directory(&self) -> Result<Vec<FileRename>> {
        if !self.config.directory.exists() {
            return Err(anyhow::anyhow!("Directory does not exist: {:?}", self.config.directory));
        }        let files: Vec<_> = fs::read_dir(&self.config.directory)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();

        let mut proposed_renames = Vec::new();
        
        match self.config.file_type {
            FileType::TvShow => {
                let mut files_for_flexible = Vec::new();
                
                for filename in &files {
                    if let Some(rename) = self.process_file_standard(filename)? {
                        proposed_renames.push(rename);
                    } else {
                        files_for_flexible.push(filename.clone());
                    }
                }

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

            let sanitized_title = sanitize_filename(&episode_title.replace(' ', "_"));            let year_part = self.config.year.as_ref()
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
            }));        }

        Ok(None)
    }pub fn process_file_with_manual_season(&self, filename: &str, manual_season: u32) -> Result<Option<FileRename>> {
        let mut file_rename_result = self.process_file_standard(filename)?;
        if file_rename_result.is_none() {
            file_rename_result = self.process_file_flexible(filename)?;
        }
        
        if file_rename_result.is_none() {
            file_rename_result = self.process_file_movie(filename)?;
        }
        
        if let Some(mut file_rename) = file_rename_result {
            if file_rename.episode_number > 0 {
                let extension = std::path::Path::new(filename)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("mkv");
                    
                let sanitized_title = sanitize_filename(&file_rename.episode_title.replace(' ', "_"));
                
                let season_episode = format!("S{:02}E{:02}", manual_season, file_rename.episode_number);
                
                let new_name = if let Some(year) = &self.config.year {
                    format!("{}_({}({}).{}", sanitized_title, season_episode, year, extension)
                } else {
                    format!("{}_({}).{}", sanitized_title, season_episode, extension)
                };
                
                file_rename.new_name = new_name;
                file_rename.needs_rename = filename != &file_rename.new_name;
                file_rename.season_number = manual_season;
            }
            
            return Ok(Some(file_rename));
        }
        
        Ok(None)
    }    pub fn process_file_movie(&self, filename: &str) -> Result<Option<FileRename>> {
        if let Some(captures) = self.movie_pattern.captures(filename) {
            let raw_title = captures.name("title").unwrap().as_str();
            let extension = captures.name("extension").unwrap().as_str();
            let suffix = captures.name("suffix").map(|s| s.as_str()).unwrap_or("");
            
            let cleaned_title = self.clean_movie_title(raw_title, suffix);
            
            if cleaned_title.is_empty() {
                return Ok(None);
            }
            
            let sanitized_title = sanitize_filename(&cleaned_title.replace(' ', "_"));
            
            let year_part = self.config.year.as_ref()
                .map(|y| format!("_({})", y))
                .unwrap_or_default();
                
            let new_name = format!("{}{}.{}", sanitized_title, year_part, extension);
            
            let file_rename = FileRename {
                original_path: self.config.directory.join(filename),
                original_name: filename.to_string(),
                new_name: new_name.clone(),
                episode_title: cleaned_title,
                episode_number: 0,
                season_number: 1,
                needs_rename: filename != new_name,
            };
            
            return Ok(Some(file_rename));
        }
        
        Ok(None)
    }fn clean_movie_title(&self, title: &str, suffix: &str) -> String {
        let mut cleaned = title.trim().to_string();
        
        let prefixes = ["watch", "download", "stream"];
        for prefix in &prefixes {
            let pattern = format!("^{}", regex::escape(prefix));
            if let Ok(re) = Regex::new(&format!("(?i){}", pattern)) {
                cleaned = re.replace(&cleaned, "").trim().to_string();
            }
        }
        
        let quality_indicators = [
            "1080p", "720p", "480p", "4k", "hd", "bluray", "blu-ray", "dvdrip", 
            "webrip", "web-dl", "hdtv", "x264", "x265", "h264", "h265", "xvid",
            "aac", "ac3", "dts", "5.1", "7.1", "atmos", "hdr", "dolby",
            "yify", "rarbg", "ettv", "eztv", "torrent", "bit", "hexa watch", "hexa"
        ];
        
        if !suffix.is_empty() {
            let suffix_words: Vec<&str> = suffix.split(&[' ', '.', '-', '_'][..])
                .filter(|word| !word.is_empty())
                .filter(|word| {
                    let word_lower = word.to_lowercase();
                    !quality_indicators.iter().any(|indicator| word_lower.contains(indicator)) &&
                    !word_lower.starts_with("watch")
                })
                .collect();
            
            if !suffix_words.is_empty() && suffix_words.len() <= 3 {
                let suffix_text = suffix_words.join(" ");
                if !cleaned.to_lowercase().contains(&suffix_text.to_lowercase()) {
                    cleaned = format!("{} {}", cleaned, suffix_text);
                }
            }
        }
        
        // Remove quality indicators
        for indicator in &quality_indicators {
            let pattern = format!(r"(?i)\b{}\b", regex::escape(indicator));
            if let Ok(re) = Regex::new(&pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }
        
        // Remove "watch" patterns at the end with optional numbers (e.g., "Watch_2", "Watch 3")
        if let Ok(watch_end_re) = Regex::new(r"(?i)\s*-?\s*(?:hexa\s*)?watch(?:_?\d+)?\s*$") {
            cleaned = watch_end_re.replace(&cleaned, "").trim().to_string();
        }
        
        // Remove year if not specified in config
        if self.config.year.is_none() {
            if let Ok(year_regex) = Regex::new(r"\b(19\d{2}|20\d{2})\b") {
                if let Some(_year_match) = year_regex.find(&cleaned) {
                    cleaned = year_regex.replace(&cleaned, "").trim().to_string();
                }
            }
        }
        
        // Final cleanup
        cleaned = cleaned.trim()
            .replace("  ", " ")
            .replace(" .", "")
            .replace(" -", "")
            .replace("- ", "")
            .trim_end_matches('-')
            .trim_end_matches('.')
            .trim()
            .to_string();
        
        // Capitalize each word
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
    }    fn extract_episode_title_from_suffix(&self, suffix: &str) -> String {
        let cleaned = suffix.trim().to_string();
          let quality_indicators = [
            "1080p", "720p", "480p", "4k", "2160p", "hd", "fhd", "uhd",
            "x264", "x265", "h264", "h265", "xvid", "divx", "mpeg",
            "bluray", "blu-ray", "webrip", "web-dl", "hdtv", "dvdrip", "brrip",
            "aac", "ac3", "mp3", "dts", "flac", "dd5.1", "dd5", "dd+", "atmos",
            "5.1", "7.1", "2.0", "stereo", "mono",
            "pahe.in", "rarbg", "yify", "ettv", "eztv", "torrent", "bit",
            "hexa", "watch", "download", "stream", "720p.bluray", "1080p.bluray"
        ];
          let words: Vec<&str> = cleaned.split(&['.', '-', '_', ' '][..])
            .filter(|word| !word.is_empty())
            .filter(|word| {
                let word_lower = word.to_lowercase();
                !quality_indicators.iter().any(|indicator| {
                    word_lower == indicator.to_lowercase() || 
                    word_lower.contains(&indicator.to_lowercase())
                })
            })
            .collect();
          let mut title_words = Vec::new();
        for word in words {
            if word.to_lowercase().contains("x264") || 
               word.to_lowercase().contains("x265") ||
               word.to_lowercase().contains("bluray") ||
               word.to_lowercase().contains("1080p") ||
               word.to_lowercase().contains("720p") ||
               word.len() < 2 {
                break;
            }
            title_words.push(word);
            
            if title_words.len() >= 3 {
                break;
            }
        }
        
        if title_words.is_empty() {
            return "Episode".to_string();
        }
          let title = title_words.join(" ");
        
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
                error_message: Some(e.to_string()),            }
        }
    }    pub fn process_file_with_year(&self, filename: &str, year: Option<String>) -> Result<Option<FileRename>> {
        let mut temp_config = self.config.clone();
        temp_config.year = year;
        
        let temp_engine = RenameEngine {
            config: temp_config,
            imdb_titles: self.imdb_titles.clone(),
            standard_pattern: self.standard_pattern.clone(),
            flexible_pattern: self.flexible_pattern.clone(),
            movie_pattern: self.movie_pattern.clone(),
        };
          match self.config.file_type {
            FileType::TvShow => {
                if let Some(file_rename) = temp_engine.process_file_standard(filename)? {
                    return Ok(Some(file_rename));
                } else if let Some(file_rename) = temp_engine.process_file_flexible(filename)? {
                    return Ok(Some(file_rename));
                }
            },
            FileType::Movie => {
                if let Some(file_rename) = temp_engine.process_file_movie(filename)? {
                    return Ok(Some(file_rename));
                }
            }
        }
        
        Ok(None)
    }
}

pub fn sanitize_filename(filename: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();
    re.replace_all(filename, "_").to_string()
}

pub fn extract_season_from_directory(dir_name: &str) -> Option<u32> {
    let patterns = [
        r"s(?:eason\s*)?(\d+)",           
        r"(?:season\s+)(\d+)",            
        r"(\d+)(?:st|nd|rd|th)\s*season", 
        r"series\s*(\d+)",                
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

pub fn extract_season_from_filename(filename: &str) -> Option<u32> {
    let patterns = [
        r"S(\d{1,2})E\d{2}",              
        r"(?:season\s*)?(\d+)x\d{2}",     
        r"s(\d+)e\d+",                    
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

    let html = response.text().await?;    let document = Html::parse_document(&html);
    
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
                    }                } else if !text.trim().is_empty() && !text.contains("S.") {
                    results.push(text.trim().to_string());
                }
            }
        }
        
        if !results.is_empty() {
            break;        }
    }

    let mut unique_results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    for result in results {
        if seen.insert(result.clone()) {
            unique_results.push(result);
        }
    }

    Ok(unique_results)
}

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
    }    pub fn season(mut self, season: String) -> Self {
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
          let (season, season_num) = if file_type == FileType::TvShow {
            let season = self.season
                .ok_or_else(|| anyhow::anyhow!("Season is required for TV shows"))?;
            let season_num = self.season_num
                .ok_or_else(|| anyhow::anyhow!("Season number is required for TV shows"))?;
            (season, season_num)
        } else {
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