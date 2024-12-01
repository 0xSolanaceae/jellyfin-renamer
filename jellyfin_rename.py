import os
import re

def rename_files():
    directory = input("Please enter the directory path for the files to be renamed: ")
    if not os.path.isdir(directory):
        print(f"The directory '{directory}' does not exist.")
        return

    season = "S04"
    year = "2024"

    files = os.listdir(directory)

    pattern = re.compile(r'(.+)\.S\d+E(\d+)\.(.+?)\.?1080p\.(BluRay|WEB-HD|WEB-DL)\.x(264|265)\..+')
    proposed_renames = []

    for filename in files:
        match = pattern.match(filename)
        if match:
            episode_name = match.group(3).replace('.', ' ').replace(' ', '_')
            episode_number = match.group(2)
            file_extension = os.path.splitext(filename)[1]
            new_name = f"{episode_name}_{season}E{episode_number}_({year}){file_extension}"
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
        else:
            print("No files were renamed.")
    else:
        print("No files matched the expected pattern.")

if __name__ == "__main__":
    rename_files()