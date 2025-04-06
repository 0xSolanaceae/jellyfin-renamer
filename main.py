import os
import re

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
    
    year = input("Enter year (leave blank if none): ")

    files = os.listdir(directory)
    pattern = re.compile(
        r'^S(?P<season>\d{2})E(?P<episode>\d{2})\.(?P<title>.*?)\.(?P<resolution>\d{3,4}p)\.'
        r'(?P<source>[\w-]+)\.(?P<codec>x\d{3})\.(?P<bit_depth>\d{2}Bit)\.(?P<audio>\d+CH)-(?P<group>.*?)\.(?P<extension>mkv|mp4)$',
        re.IGNORECASE
    )
    proposed_renames = []

    for filename in files:
        if match := pattern.match(filename):
            episode_number = match['episode']
            episode_title = match['title'].replace('.', '_')
            file_extension = f".{match['extension']}"

            new_name = f"{episode_title}_{season}E{episode_number}{f'({year})' if year else ''}{file_extension}"
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

    # Option to try with a more flexible pattern
    if not proposed_renames:
        retry = input("\nWould you like to try with a more flexible pattern? (y/n): ")
        if retry.lower() == 'y':
            flexible_pattern = re.compile(
                r'^S(?P<season>\d{2})E(?P<episode>\d{2})\.(?P<title>.*?)\.(?P<resolution>\d{3,4}p).*\.(?P<extension>mkv|mp4)$',
                re.IGNORECASE
            )
            proposed_renames = []
            
            for filename in files:
                if match := flexible_pattern.match(filename):
                    episode_number = match['episode']
                    episode_title = match['title'].replace('.', '_')
                    file_extension = f".{match['extension']}"
                    
                    new_name = f"{episode_title}_{season}E{episode_number}{f'({year})' if year else ''}{file_extension}"
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