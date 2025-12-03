#!/bin/bash
cd /path/to/Projects/SpotifyDownloader/spotify-downloader-gui
export PATH="$HOME/.nvm/versions/node/v24.8.0/bin:$HOME/.cargo/bin:$PATH"
npm run tauri dev
