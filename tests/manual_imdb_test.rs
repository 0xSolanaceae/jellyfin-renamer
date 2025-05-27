use std::io::{self, Write};

use jellyfin_rename::rename_engine::scrape_imdb_episodes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IMDb Scraper Manual Test ===");
    println!("This tool lets you test the IMDb scraper with any show you want.\n");
    
    loop {
        println!("Enter an IMDb ID (e.g., tt0903747 for Breaking Bad) or 'quit' to exit:");
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let imdb_id = input.trim();
        
        if imdb_id.eq_ignore_ascii_case("quit") || imdb_id.eq_ignore_ascii_case("q") {
            break;
        }
        
        if imdb_id.is_empty() {
            continue;
        }
        
        println!("Enter season number (default: 1):");
        print!("> ");
        io::stdout().flush()?;
        
        let mut season_input = String::new();
        io::stdin().read_line(&mut season_input)?;
        let season_str = season_input.trim();
        
        let season = if season_str.is_empty() {
            1
        } else {
            match season_str.parse::<u32>() {
                Ok(s) => s,
                Err(_) => {
                    println!("Invalid season number, using 1");
                    1
                }
            }
        };
        
        println!("\nFetching episodes for IMDb ID: {} (Season {})...", imdb_id, season);
        
        match scrape_imdb_episodes(imdb_id, Some(season)).await {
            Ok(episodes) => {
                if episodes.is_empty() {
                    println!(" No episodes found. This could mean:");
                    println!("   - Invalid IMDb ID");
                    println!("   - Invalid season number");
                    println!("   - IMDb page structure changed");
                    println!("   - Network connectivity issues");
                } else {
                    println!(" Successfully fetched {} episodes:", episodes.len());
                    for (i, episode) in episodes.iter().enumerate() {
                        println!("   Episode {:2}: {}", i + 1, episode);
                    }
                }
            }
            Err(e) => {
                println!(" Error fetching episodes: {}", e);
                println!("   This could be due to:");
                println!("   - Invalid IMDb ID (404 error)");
                println!("   - Network connectivity issues");
                println!("   - IMDb blocking the request");
            }        }
        
        println!("\n{}", "=".repeat(50));
    }
    
    println!("Goodbye!");
    Ok(())
}
