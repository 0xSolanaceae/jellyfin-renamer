// Quick IMDb test - run with: cargo run --bin quick_imdb_test

use reqwest;
use scraper::{Html, Selector};
use anyhow::{Result, Context};

async fn scrape_imdb_episodes(imdb_id: &str, season: Option<u32>) -> Result<Vec<String>> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("IMDb Scraper Test\n");
    
    // Test with popular shows
    let test_cases = [
        ("Breaking Bad Season 1", "tt0903747", 1),
        ("The Office Season 1", "tt0386676", 1),
        ("Stranger Things Season 1", "tt4574334", 1),
        ("Game of Thrones Season 1", "tt0944947", 1),
    ];
    
    for (show_name, imdb_id, season) in &test_cases {
        println!("📺 Testing: {}", show_name);
        println!("   IMDb ID: {}", imdb_id);
        
        match scrape_imdb_episodes(imdb_id, Some(*season)).await {
            Ok(episodes) => {
                if episodes.is_empty() {
                    println!("   No episodes found");
                } else {
                    println!("   Found {} episodes:", episodes.len());
                    for (i, episode) in episodes.iter().take(3).enumerate() {
                        println!("      Episode {}: {}", i + 1, episode);
                    }
                    if episodes.len() > 3 {
                        println!("      ... and {} more episodes", episodes.len() - 3);
                    }
                }
            }
            Err(e) => {
                println!("   Error: {}", e);
            }
        }
        println!();
    }
    
    println!("Test with your own show:");
    println!("   Find the IMDb URL (e.g., https://www.imdb.com/title/tt0903747/)");
    println!("   Extract the ID (e.g., tt0903747)");
    println!("   Run: cargo test test_imdb_scraper_breaking_bad -- --nocapture");
    
    Ok(())
}
