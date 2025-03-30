import os
import re

def rename_files():
    directory = input("Please enter the directory path for the files to be renamed: ")
    print("")
    if not os.path.isdir(directory):
        print(f"The directory '{directory}' does not exist.")
        return

    season = "S01"
    year = "2022"

    files = os.listdir(directory)

    pattern = re.compile(
        r'^(?P<show_name>.+?)\.S(?P<season>\d{2})E(?P<episode>\d{2})\.'
        r'(?P<episode_title>.+?)\.'
        r'(?P<resolution>\d{3,4}p)\.BluRay\.x265\.(?P<audio>.+?)\.'
        r'(?P<group>.+?)\.(?P<extension>mkv|mp4)$',
        re.IGNORECASE
    )
    proposed_renames = []

    for filename in files:
        if match := pattern.match(filename):
            show_name = match['show_name'].replace('.', ' ').replace(' ', '_')
            episode_number = match['episode']
            episode_title = (
                (match['episode_title'] or "")
                .replace('.', ' ')
                .replace(' ', '_')
            )
            file_extension = f".{match['extension']}"

            new_name = f"{show_name}_{season}E{episode_number}"
            if episode_title:
                new_name += f"_{episode_title}"
            new_name += f"_({year}){file_extension}"
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

if __name__ == "__main__":
    rename_files()