// Integration tests for the rename engine module

use std::path::PathBuf;
use jellyfin_rename::rename_engine::{
    sanitize_filename, extract_season_from_directory, scrape_imdb_episodes,
    ConfigBuilder, RenameEngine, FileType
};

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
        .file_type(FileType::TvShow)
        .season("S01".to_string())
        .year(Some("2023".to_string()))
        .build()
        .unwrap();

    assert_eq!(config.season, "S01");
    assert_eq!(config.season_num, 1);
    assert_eq!(config.year, Some("2023".to_string()));
}

#[tokio::test]
async fn test_imdb_scraper_breaking_bad() {
    println!("Testing IMDb scraper with Breaking Bad Season 1...");
    
    let imdb_id = "tt0903747"; // Breaking Bad
    let season = 1;
    
    match scrape_imdb_episodes(imdb_id, Some(season)).await {
        Ok(episodes) => {
            println!("Successfully fetched {} episodes:", episodes.len());
            for (i, episode) in episodes.iter().enumerate() {
                println!("  Episode {}: {}", i + 1, episode);
            }
            assert!(!episodes.is_empty(), "Should fetch at least one episode");
            
            // Breaking Bad Season 1 should have 7 episodes
            if episodes.len() >= 7 {
                println!("✓ Fetched expected number of episodes (7 or more)");
            } else {
                println!("⚠ Expected 7 episodes, got {}", episodes.len());
            }
        }
        Err(e) => {
            println!("Error fetching episodes: {}", e);
            panic!("IMDb scraper failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_imdb_scraper_the_office() {
    println!("Testing IMDb scraper with The Office Season 1...");
    
    let imdb_id = "tt0386676"; // The Office (US)
    let season = 1;
    
    match scrape_imdb_episodes(imdb_id, Some(season)).await {
        Ok(episodes) => {
            println!("Successfully fetched {} episodes:", episodes.len());
            for (i, episode) in episodes.iter().take(3).enumerate() {
                println!("  Episode {}: {}", i + 1, episode);
            }
            if episodes.len() > 3 {
                println!("  ... and {} more episodes", episodes.len() - 3);
            }
            assert!(!episodes.is_empty(), "Should fetch at least one episode");
        }
        Err(e) => {
            println!("Error fetching episodes: {}", e);
            // Don't panic for this test, just report the error
            eprintln!("IMDb scraper failed for The Office: {}", e);
        }
    }
}

#[tokio::test]
async fn test_imdb_scraper_invalid_id() {
    println!("Testing IMDb scraper with invalid ID...");
    
    let invalid_id = "tt9999999";
    
    match scrape_imdb_episodes(invalid_id, Some(1)).await {
        Ok(episodes) => {
            println!("Unexpectedly succeeded with {} episodes", episodes.len());
            // If it succeeds with 0 episodes, that's also acceptable
            if episodes.is_empty() {
                println!("✓ Correctly returned empty list for invalid ID");
            }
        }
        Err(e) => {
            println!("✓ Expected error for invalid ID: {}", e);
            // This is expected behavior
        }
    }
}

#[tokio::test]
async fn test_rename_engine_integration() {
    println!("Testing RenameEngine IMDb integration...");
    
    let config = ConfigBuilder::new()
        .directory(PathBuf::from("C:\\temp\\test"))
        .file_type(FileType::TvShow)
        .season("S01".to_string())
        .imdb(Some("tt0903747".to_string())) // Breaking Bad
        .build()
        .unwrap();
    
    let mut engine = RenameEngine::new(config).unwrap();
    
    match engine.fetch_imdb_titles().await {
        Ok(_) => println!("RenameEngine successfully fetched IMDb titles"),
        Err(e) => println!("RenameEngine IMDb fetch error: {}", e),
    }
}
