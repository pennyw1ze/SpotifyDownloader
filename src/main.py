import subprocess
import os
from pathlib import Path

# Default download path
DEFAULT_DOWNLOAD_PATH = "../Music"


def download_spotify_content(url, content_type="track", threads=4, download_path=None, show_output=False):
    """
    Download Spotify content (track, playlist, or album)
    
    Args:
        url (str): Spotify URL
        content_type (str): Type of content - "track", "playlist", or "album"
        threads (int): Number of parallel downloads (only used for playlists/albums)
        download_path (str): Path to download to (uses default if None)
        show_output (bool): If True, show spotdl output in real-time (faster for GUI)
    
    Returns:
        dict: {"success": bool, "message": str, "error": str}
    """
    if download_path is None:
        download_path = DEFAULT_DOWNLOAD_PATH
    
    try:
        # Ensure download directory exists
        os.makedirs(download_path, exist_ok=True)
        
        # Build spotdl command
        cmd = ["spotdl", "--format", "mp3"]
        
        # Add threads for playlists and albums
        if content_type in ['playlist', 'album', '2', '3']:
            cmd.extend(["--threads", str(threads)])
        
        cmd.append(url)
        
        # Run spotdl command
        # For GUI, don't capture output (much faster!)
        if show_output:
            result = subprocess.run(
                cmd,
                cwd=download_path,
                check=True
            )
        else:
            result = subprocess.run(
                cmd,
                cwd=download_path,
                check=True,
                capture_output=True,
                text=True
            )
        
        return {
            "success": True,
            "message": f"{content_type.capitalize()} downloaded successfully!",
            "error": None
        }
        
    except subprocess.CalledProcessError as e:
        error_msg = e.stderr if e.stderr else str(e)
        return {
            "success": False,
            "message": None,
            "error": f"Download failed: {error_msg}"
        }
    except Exception as e:
        return {
            "success": False,
            "message": None,
            "error": str(e)
        }


def download_spotify_mp3():
    print("Spotify to MP3 Downloader")

    content_type = input("What do you want to download? (Enter '1 for track', '2 for playlist', '3 for album', '4 to exit'): ").strip().lower()
    while content_type not in ['1', '2', '3', '4']:
        content_type = input("Invalid choice. Please enter '1', '2', '3', or '4'.").strip().lower()
    
    if content_type == '4':
        print("Exiting the program.")
        return
    

    spotify_url = input(f"Enter the Spotify {content_type} URL: ").strip()

    # Check for valid input:
    while not not spotify_url.startswith("https://open.spotify.com/"):
        spotify_url = input("Invalid URL. Please enter a valid Spotify URL: ").strip()

    if content_type == '1':
        print(f"\nDownloading track as MP3...")
        result = download_spotify_content(spotify_url, "track", download_path=DEFAULT_DOWNLOAD_PATH)
        if result["success"]:
            print(f"\n✓ {result['message']}\n")
        else:
            print(f"\n✗ {result['error']}\n")
            
    elif content_type in ['2', '3']:
        # For playlists and albums, use spotdl's built-in threading for better performance
        num_threads = input("Enter number of parallel downloads (default: 4, max recommended: 8): ").strip()
        if not num_threads or not num_threads.isdigit():
            num_threads = "4"
        else:
            num_threads = str(min(int(num_threads), 16))  # Cap at 16 to avoid issues
        
        content_name = "playlist" if content_type == '2' else "album"
        print(f"\nDownloading {content_name} as MP3 using {num_threads} parallel threads...")
        
        result = download_spotify_content(spotify_url, content_name, int(num_threads), DEFAULT_DOWNLOAD_PATH)
        if result["success"]:
            print(f"\n✓ {result['message']}\n")
        else:
            print(f"\n✗ {result['error']}\n")

if __name__ == "__main__":
    download_spotify_mp3()
