import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

// Types
type ContentType = "track" | "playlist" | "album";
type StatusType = "info" | "success" | "error" | "";

interface ProgressPayload {
  percent: number;
  message: string;
  current_track: number;
  total_tracks: number;
  speed: string;
}

interface ProgressState {
  visible: boolean;
  percent: number;
  text: string;
  currentTrack: number;
  totalTracks: number;
  speed: string;
}

function App() {
  // State
  const [contentType, setContentType] = useState<ContentType>("track");
  const [spotifyUrl, setSpotifyUrl] = useState("");
  const [threads, setThreads] = useState(4);
  const [downloadPath, setDownloadPath] = useState("");
  const [isDownloading, setIsDownloading] = useState(false);
  const [status, setStatus] = useState({ message: "", type: "" as StatusType });
  const [progress, setProgress] = useState<ProgressState>({
    visible: false,
    percent: 0,
    text: "",
    currentTrack: 0,
    totalTracks: 0,
    speed: "",
  });

  // Initialize - get default download path and set up event listeners
  useEffect(() => {
    async function init() {
      try {
        const path = await invoke<string>("get_download_path");
        setDownloadPath(path);
      } catch (e) {
        console.error("Failed to get download path:", e);
      }
    }
    init();

    // Listen for progress updates from Rust backend
    const unlisten = listen<ProgressPayload>("download-progress", (event) => {
      setProgress({
        visible: true,
        percent: event.payload.percent,
        text: event.payload.message,
        currentTrack: event.payload.current_track,
        totalTracks: event.payload.total_tracks,
        speed: event.payload.speed,
      });
    });

    // Cleanup listener on unmount
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Show status message
  const showStatus = (message: string, type: StatusType) => {
    setStatus({ message, type });
    if (type === "success" || type === "error") {
      setTimeout(() => setStatus({ message: "", type: "" }), 5000);
    }
  };

  // Browse for folder
  const handleBrowse = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: downloadPath,
        title: "Select Download Location",
      });
      if (selected) {
        setDownloadPath(selected as string);
        showStatus(`Download location updated: ${selected}`, "success");
      }
    } catch (e) {
      console.error("Error selecting folder:", e);
      showStatus(`Error: ${e}`, "error");
    }
  };

  // Download handler
  const handleDownload = async () => {
    // Validation
    if (!spotifyUrl.trim()) {
      showStatus("Please enter a Spotify URL", "error");
      return;
    }

    if (!spotifyUrl.startsWith("https://open.spotify.com/")) {
      showStatus("Please enter a valid Spotify URL", "error");
      return;
    }

    setIsDownloading(true);
    setProgress({ visible: true, percent: 0, text: "Initializing...", currentTrack: 0, totalTracks: 0, speed: "" });

    try {
      const result = await invoke<string>("download_content", {
        url: spotifyUrl,
        contentType,
        threads,
        downloadPath,
      });

      showStatus(`✓ ${result}`, "success");
      setSpotifyUrl("");
      
      setTimeout(() => {
        setProgress({ visible: false, percent: 0, text: "", currentTrack: 0, totalTracks: 0, speed: "" });
      }, 1500);
    } catch (e) {
      showStatus(`✗ ${e}`, "error");
      setProgress({ visible: false, percent: 0, text: "", currentTrack: 0, totalTracks: 0, speed: "" });
    } finally {
      setIsDownloading(false);
    }
  };

  // Cancel handler
  const handleCancel = async () => {
    try {
      await invoke("cancel_download");
      showStatus("Download cancelled", "info");
      setIsDownloading(false);
      setProgress({ visible: false, percent: 0, text: "", currentTrack: 0, totalTracks: 0, speed: "" });
    } catch (e) {
      showStatus(`Failed to cancel: ${e}`, "error");
    }
  };

  return (
    <div className="container">
      {/* Header */}
      <header className="header">
        <div className="logo">
          <svg className="spotify-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 0C5.4 0 0 5.4 0 12s5.4 12 12 12 12-5.4 12-12S18.66 0 12 0zm5.521 17.34c-.24.359-.66.48-1.021.24-2.82-1.74-6.36-2.101-10.561-1.141-.418.122-.779-.179-.899-.539-.12-.421.18-.78.54-.9 4.56-1.021 8.52-.6 11.64 1.32.42.18.479.659.301 1.02zm1.44-3.3c-.301.42-.841.6-1.262.3-3.239-1.98-8.159-2.58-11.939-1.38-.479.12-1.02-.12-1.14-.6-.12-.48.12-1.021.6-1.141C9.6 9.9 15 10.561 18.72 12.84c.361.181.54.78.241 1.2zm.12-3.36C15.24 8.4 8.82 8.16 5.16 9.301c-.6.179-1.2-.181-1.38-.721-.18-.601.18-1.2.72-1.381 4.26-1.26 11.28-1.02 15.721 1.621.539.3.719 1.02.419 1.56-.299.421-1.02.599-1.559.3z" fill="currentColor"/>
          </svg>
          <h1>Spotify Downloader</h1>
        </div>
        <p className="subtitle">Download your favorite tracks, playlists, and albums</p>
      </header>

      {/* Main Content */}
      <main className="main-content">
        {/* Content Type Selection */}
        <div className="form-group">
          <label htmlFor="contentType">Content Type</label>
          <select
            id="contentType"
            className="select-input"
            value={contentType}
            onChange={(e) => setContentType(e.target.value as ContentType)}
          >
            <option value="track">Track</option>
            <option value="playlist">Playlist</option>
            <option value="album">Album</option>
          </select>
        </div>

        {/* URL Input */}
        <div className="form-group">
          <label htmlFor="spotifyUrl">Spotify URL</label>
          <input
            type="text"
            id="spotifyUrl"
            className="text-input"
            placeholder="https://open.spotify.com/..."
            value={spotifyUrl}
            onChange={(e) => setSpotifyUrl(e.target.value)}
          />
        </div>

        {/* Threads Selection (only for playlists/albums) */}
        {(contentType === "playlist" || contentType === "album") && (
          <div className="form-group">
            <label htmlFor="threads">Parallel Downloads</label>
            <div className="slider-container">
              <input
                type="range"
                id="threads"
                className="slider"
                min={1}
                max={8}
                value={threads}
                onChange={(e) => setThreads(Number(e.target.value))}
              />
              <span className="slider-value">{threads}</span>
            </div>
          </div>
        )}

        {/* Download Path */}
        <div className="form-group">
          <label htmlFor="downloadPath">Download Location</label>
          <div className="path-selector">
            <input
              type="text"
              id="downloadPath"
              className="text-input path-input"
              placeholder="Click Browse or type path..."
              value={downloadPath}
              onChange={(e) => setDownloadPath(e.target.value)}
            />
            <button className="btn-secondary" onClick={handleBrowse}>
              Browse
            </button>
          </div>
        </div>

        {/* Download Button */}
        <button
          className="btn-primary"
          onClick={handleDownload}
          disabled={isDownloading}
        >
          <svg className="btn-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 16L7 11L8.4 9.55L11 12.15V4H13V12.15L15.6 9.55L17 11L12 16ZM6 20C5.45 20 4.979 19.804 4.587 19.412C4.195 19.02 3.999 18.549 4 18V15H6V18H18V15H20V18C20 18.55 19.804 19.021 19.412 19.413C19.02 19.805 18.549 20.001 18 20H6Z" fill="currentColor"/>
          </svg>
          {isDownloading ? "Downloading..." : "Download"}
        </button>

        {/* Status Message */}
        {status.message && (
          <div className={`status-message show ${status.type}`}>
            {status.message}
          </div>
        )}

        {/* Progress Bar */}
        {progress.visible && (
          <div className="progress-container">
            <div className="progress-header">
              <span className="progress-text">{progress.text}</span>
              {progress.totalTracks > 0 && (
                <span className="progress-stats">
                  {progress.currentTrack}/{progress.totalTracks}
                  {progress.speed && <span className="progress-speed"> • {progress.speed}</span>}
                </span>
              )}
            </div>
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${progress.percent}%` }}
              />
            </div>
            <div className="progress-footer">
              <div className="progress-percent">{progress.percent}%</div>
              {isDownloading && progress.percent < 100 && (
                <button className="btn-cancel" onClick={handleCancel}>
                  Cancel
                </button>
              )}
            </div>
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="footer">
        <p>Powered by spotdl</p>
      </footer>
    </div>
  );
}

export default App;
