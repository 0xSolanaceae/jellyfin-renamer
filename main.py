import os
import re
import requests
from bs4 import BeautifulSoup

def scrape_imdb_episodes(imdb_id, season=None):
    url = f"https://www.imdb.com/title/{imdb_id}/episodes"
    if season:
        url += f"?season={season}"
    
    headers = {
        "User-Agent": "Mozilla/5.0"
    }
    
    try:
        response = requests.get(url, headers=headers)
        response.raise_for_status()
        
        soup = BeautifulSoup(response.text, 'html.parser')
        episodes = soup.select('div.ipc-title.ipc-title--base.ipc-title--title')
        
        results = []
        for episode in episodes:
            title_element = episode.select_one('.ipc-title__text')
            if title_element and '∙' in title_element.text:
                title = title_element.text.strip().split('∙')[-1].strip()
                results.append(title)
        
        return results
    
    except Exception as e:
        print(f"Error fetching IMDb data: {e}")
        return []

def sanitize_filename(filename):
    return re.sub(r'[<>:"/\\|?*]', '_', filename)

def rename_files():
    directory = input("Please enter the directory path for the files to be renamed: ")
    print("")
    if not os.path.isdir(directory):
        print(f"The directory '{directory}' does not exist.")
        return

    dir_name = os.path.basename(directory)
    season_match = re.search(r's(\d+)', dir_name, re.IGNORECASE)
    if season_match:
        season_num = int(season_match.group(1))
        season = f"S{season_num:02d}"
    else:
        season = input("Enter season number (e.g., S01): ")
        if not season.startswith('S'):
            season = f"S{int(season):02d}"
        season_num = int(season[1:])

    year = "" # input("Enter year (leave blank if none): ")

    use_imdb = input("Would you like to fetch episode titles from IMDb? (y/n): ").lower() == 'y'
    imdb_titles = []

    if use_imdb:
        imdb_id = input("Enter IMDb ID (e.g., tt0944947): ").strip()
        print(f"Fetching episode titles for {season} from IMDb...")
        imdb_titles = scrape_imdb_episodes(imdb_id, season_num)
        if not imdb_titles:
            print("Could not fetch episode titles. Proceeding without IMDb titles.")
        else:
            print(f"Fetched {len(imdb_titles)} episode titles.")

    files = sorted(os.listdir(directory))
    pattern = re.compile(
        r'(?i)(?P<title>.*?)S(?P<season>\d{1,2})E(?P<episode>\d{2})(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi)$'
    )

    proposed_renames = []

    for filename in files:
        if match := pattern.match(filename):
            episode_number = int(match['episode'])
            season_number = match['season']
            
            if imdb_titles and episode_number <= len(imdb_titles):
                episode_title = imdb_titles[episode_number - 1]
            else:
                title_parts = match['suffix'].split('.')
                meaningful_parts = []
                for part in title_parts:
                    if part.lower() in ['1080p', '720p', 'bluray', 'x264', 'x265', 'web-dl', 'webrip']:
                        break
                    if part:
                        meaningful_parts.append(part)
                
                episode_title = ' '.join(meaningful_parts)
            
            episode_title = episode_title.replace(' ', '_')
            episode_title = sanitize_filename(episode_title)
            file_extension = f".{match['extension']}"
            
            season_episode = f"S{int(season_number):02d}E{episode_number:02d}"
            new_name = f"{episode_title}_({season_episode}){file_extension}"

            if filename == new_name:
                continue

            proposed_renames.append((filename, new_name))
        else:
            print(f"Filename '{filename}' does not match the expected pattern")

    if proposed_renames:
        print("\nProposed renames:")
        for old_name, new_name in proposed_renames:
            print(f"'{old_name}' --> '{new_name}'")

        confirm = input("\nDo you want to rename these files? (y/n): ")
        if confirm.lower() == 'y':
            for old_name, new_name in proposed_renames:
                os.rename(os.path.join(directory, old_name), os.path.join(directory, new_name))
            print("Files have been renamed successfully.")
        else:
            print("No files were renamed.")
    else:
        print("No files matched the expected pattern.")

    if not proposed_renames:
        retry = input("\nWould you like to try with a more flexible pattern? (y/n): ")
        if retry.lower() == 'y':
            flexible_pattern = re.compile(
                r'(?i)(?P<title>.*?)\b(?P<season>\d{1,2})x(?P<episode>\d{2})\b(?P<suffix>.*)\.(?P<extension>mkv|mp4|avi)$'
            )
            proposed_renames = []

            for filename in files:
                if match := flexible_pattern.match(filename):
                    episode_number = int(match['episode'])
                    episode_title = imdb_titles[episode_number - 1].replace(' ', '_') if imdb_titles and episode_number <= len(imdb_titles) else match['title'].replace('.', '_')
                    file_extension = f".{match['extension']}"
                    new_name = f"{episode_title}_{season}E{episode_number:02d}{f'({year})' if year else ''}{file_extension}"
                    proposed_renames.append((filename, new_name))

            if proposed_renames:
                print("\nProposed renames with flexible pattern:")
                for old_name, new_name in proposed_renames:
                    print(f"'{old_name}' --> '{new_name}'")

                confirm = input("\nDo you want to rename these files? (y/n): ")
                if confirm.lower() == 'y':
                    for old_name, new_name in proposed_renames:
                        os.rename(os.path.join(directory, old_name), os.path.join(directory, new_name))
                    print("Files have been renamed successfully.")
                else:
                    print("No files were renamed.")
            else:
                print("No files matched the flexible pattern either.")

if __name__ == "__main__":
    rename_files()
