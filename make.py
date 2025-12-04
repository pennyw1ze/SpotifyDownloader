# File to handle build and installation processes
import os
import shutil, subprocess

# Replace path in a given file
def replace_path(filename, old_path, new_path):
    with open(filename, 'r') as file:
        content = file.read()
    
    # Example: Replace a placeholder with the new path
    content = content.replace(old_path, new_path)
    
    with open(filename, 'w') as file:
        file.write(content)

# Replace path to include current directory
def add_path():
    current_dir = os.path.dirname(os.path.abspath(__file__))

    # Replace paths in run-app.sh
    print("Replacing paths in run-app.sh...")
    replace_path("run-app.sh", '/path/to/Projects/SpotifyDownloader', current_dir)

    # Replace path in spotify-downloader-sample.desktop
    print("Replacing paths in spotify-downloader-sample.desktop...")
    replace_path("spotify-downloader-sample.desktop", '/path/to/SpotifyDownloader', current_dir)

# Main function
def main():
    print("Starting build and installation process...")
    
    # On what platform are we?
    # Supported platforms: Debian, Arch, Windows, MacOS
    platform = ""
    if os.name == 'nt':
        platform = "Windows"
    elif os.uname().sysname == 'Darwin':
        platform = "MacOS"
    else:
        # Check for Debian-based systems
        if os.path.exists('/etc/debian_version'):
            platform = "Debian"
        # Check for Arch-based systems
        elif os.path.exists('/etc/arch-release'):
            platform = "Arch"
        else:
            print("Unsupported platform. We suggest to install the application manually. Check README.md. Exiting...")
            return
    print(f"Detected platform: {platform}")

    # Check if node is installed, if not, install it
    if os.system("node -v") != 0:
        print("Node.js is not installed. Installing Node.js...")
        if platform == "Debian":
            os.system("sudo apt-get install -y nodejs npm")
        elif platform == "Arch":
            os.system("sudo pacman -S --noconfirm nodejs npm")
        elif platform == "MacOS":
            os.system("brew install node")
        elif platform == "Windows":
            print("Please install Node.js manually from https://nodejs.org/")
            return
    else:
        print("Node.js is already installed.")
        
    # Check if cargo is installed, if not, install it
    if os.system("cargo --version") != 0:
        print("Cargo is not installed. Installing Rust and Cargo...")
        if platform in ["Debian", "Arch", "MacOS"]:
            os.system("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")
            os.environ["PATH"] += os.pathsep + os.path.expanduser("~/.cargo/bin")
        elif platform == "Windows":
            print("Please install Rust and Cargo manually from https://www.rust-lang.org/tools/install and re-run the installer.")
            return
        
    # Install webkit2GTK if not present
    print("Checking for webkit2gtk...")
    if platform in ["Debian", "Arch"]:
        installed = False
        if platform == "Debian":
            # Prefer pkg-config check for the development package
            installed = (os.system("pkg-config --exists webkit2gtk-4.1") == 0)
            # fallback to checking installed package name if needed
            if not installed:
                installed = (os.system("dpkg -s libwebkit2gtk-4.1-37 > /dev/null 2>&1") == 0)
        elif platform == "Arch":
            installed = (os.system("pacman -Qi webkit2gtk > /dev/null 2>&1") == 0)

        if installed:
            print("webkit2gtk is already installed.")
        else:
            print("Installing webkit2GTK...")
            if platform == "Debian":
                os.system("sudo apt-get install -y libwebkit2gtk-4.1-dev")
            elif platform == "Arch":
                os.system("sudo pacman -S --noconfirm webkit2gtk")
    elif platform == "MacOS":
        # Homebrew check
        if os.system("brew list --versions webkit2gtk > /dev/null 2>&1") == 0:
            print("webkit2gtk is already installed via Homebrew.")
        else:
            print("Installing webkit2GTK via Homebrew...")
            os.system("brew install webkit2gtk")
    elif platform == "Windows":
        print("Please install webkit2GTK manually. Check the README.md for instructions.")
        return
    print("\n================================\n")

    # Enter or generate environment and install python requirements
    print("Setting up Python environment and installing requirements...")
    os.system("python3 -m venv venv")
    if os.name == 'nt':
        os.system(".\\venv\\Scripts\\activate && pip install -r requirements.txt")
    else:
        os.system("source ./venv/bin/activate && pip install -r requirements.txt")
    print("Python environment setup completed.")
    print("\n================================\n")
    
    # Build the Tauri application
    print("Building the Tauri application...")
    if os.name == 'nt':
        os.system(".\\venv\\Scripts\\activate && cd spotify-downloader-gui && npm install && npx tauri build")
    else:
        os.system("source ./venv/bin/activate && cd spotify-downloader-gui && npm install && npx tauri build")
    print("Tauri application build completed.")
    print("\n================================\n")
    
    print("Build and installation completed.")
    # Add path to module search
    add_path()

    # Rename the desktop file to remove '-sample'
    os.rename("spotify-downloader-sample.desktop", "spotify-downloader.desktop")
    print("Renamed desktop file to spotify-downloader.desktop")
    print("\n================================\n")

    # Give permissions to run-app.sh
    os.chmod("run-app.sh", 0o755)
    print("Set execute permissions for run-app.sh")
    print("\n================================\n")

    # Adding desktop file to applications
    choice = input("Do you want to add the application to your system applications? (y/n): ").strip().lower()
    if choice == 'y':
        current_dir = os.path.dirname(os.path.abspath(__file__))
        desktop_file_src = os.path.join(current_dir, "spotify-downloader.desktop")
        home = os.path.expanduser("~")
        user_app_dir = os.path.join(home, ".local", "share", "applications")

        if platform in ["Debian", "Arch"]:
            # make executable, copy to user applications and refresh desktop db
            try:
                os.chmod(desktop_file_src, 0o755)
            except Exception:
                print("Warning: failed to chmod the desktop file.")
            os.makedirs(user_app_dir, exist_ok=True)
            shutil.copy2(desktop_file_src, user_app_dir)
            try:
                subprocess.run(["update-desktop-database", user_app_dir], check=True)
            except Exception:
                print("Warning: update-desktop-database failed or is not available. You can run it manually.")

        elif platform == "MacOS":
            # .desktop files are not native on macOS; copy for reference and notify user
            mac_dest = os.path.join(home, "Applications")
            os.makedirs(mac_dest, exist_ok=True)
            shutil.copy2(desktop_file_src, mac_dest)
            print("Note: macOS does not use .desktop files. Copied the desktop file to ~/Applications for reference.")
            print("Create a native .app or use the README instructions to register the application in macOS.")

        elif platform == "Windows":
            # Windows doesn't use .desktop files; instruct the user
            print("Windows detected. Please create a shortcut (.lnk) to run-app.bat or register the application via the Start Menu.")
            print(f"The desktop file is available at: {desktop_file_src}")


    print("Done. Exiting ...")


if __name__ == "__main__":
    main()