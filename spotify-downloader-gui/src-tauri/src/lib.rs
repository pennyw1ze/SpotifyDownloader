// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::process::{Command, Stdio};
use std::path::Path;
use std::fs;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::thread;
use tauri::{AppHandle, Emitter};
use serde::Serialize;

// Global state for the download process
struct DownloadState {
    child_pid: AtomicU32,
    is_cancelled: AtomicBool,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            child_pid: AtomicU32::new(0),
            is_cancelled: AtomicBool::new(false),
        }
    }
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    percent: u32,
    message: String,
    current_track: u32,
    total_tracks: u32,
    speed: String,  // e.g., "2.5 songs/min"
}

/// Cancel the current download
#[tauri::command]
fn cancel_download(state: tauri::State<DownloadState>) -> Result<(), String> {
    let pid = state.child_pid.load(Ordering::SeqCst);
    if pid > 0 {
        state.is_cancelled.store(true, Ordering::SeqCst);
        
        // Kill the process group
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(format!("-{}", pid))
            .output();
        
        // Also kill the process directly as fallback
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .output();
        
        state.child_pid.store(0, Ordering::SeqCst);
        Ok(())
    } else {
        Err("No active download to cancel".to_string())
    }
}

/// Helper function to process output lines
fn process_output_line(
    line: &str,
    app: &AppHandle,
    current_track: &Arc<Mutex<u32>>,
    total_tracks: &Arc<Mutex<u32>>,
    last_percent: &Arc<Mutex<u32>>,
    start_time: &Instant,
) {
    let message = line.trim();
    if message.is_empty() {
        return;
    }

    // Use unwrap_or_else to handle poisoned mutexes gracefully
    let Ok(mut current) = current_track.lock() else { return };
    let Ok(mut total) = total_tracks.lock() else { return };
    let Ok(mut last_pct) = last_percent.lock() else { return };

    // Calculate download speed
    let elapsed_secs = start_time.elapsed().as_secs_f64();
    let speed = if *current > 0 && elapsed_secs > 0.0 {
        let songs_per_min = (*current as f64 / elapsed_secs) * 60.0;
        if songs_per_min >= 1.0 {
            format!("{:.1} songs/min", songs_per_min)
        } else {
            let secs_per_song = elapsed_secs / *current as f64;
            format!("{:.0}s/song", secs_per_song)
        }
    } else {
        "calculating...".to_string()
    };

    // Check for "Found X songs" or "Processing query" patterns
    if (message.contains("Found") && message.contains("song")) || message.contains("Processing query") {
        if let Some(count) = extract_number(message) {
            *total = count.max(1);
            let _ = app.emit("download-progress", ProgressPayload {
                percent: 10,
                message: format!("Found {} song(s), starting download...", *total),
                current_track: 0,
                total_tracks: *total,
                speed: "".to_string(),
            });
            *last_pct = 10;
        }
    }
    // Check for download progress indicators
    else if message.contains("Downloaded") {
        *current += 1;
        
        // Calculate progress: 10% for finding, 10-95% for downloading
        let download_progress = if *total > 0 {
            ((*current as f32 / *total as f32) * 85.0) as u32
        } else {
            50
        };
        let percent = (10 + download_progress).min(95);
        
        if percent > *last_pct {
            *last_pct = percent;
            let _ = app.emit("download-progress", ProgressPayload {
                percent,
                message: "Downloading...".to_string(),
                current_track: *current,
                total_tracks: *total,
                speed: speed.clone(),
            });
        }
    }
    // Check for "Skipping" messages (already downloaded)
    else if message.contains("Skipping") {
        *current += 1;
        
        let download_progress = if *total > 0 {
            ((*current as f32 / *total as f32) * 85.0) as u32
        } else {
            50
        };
        let percent = (10 + download_progress).min(95);
        
        if percent > *last_pct {
            *last_pct = percent;
            let _ = app.emit("download-progress", ProgressPayload {
                percent,
                message: "Processing...".to_string(),
                current_track: *current,
                total_tracks: *total,
                speed: speed.clone(),
            });
        }
    }
    // Check for conversion/processing
    else if message.contains("Converting") || message.contains("Processing") {
        let _ = app.emit("download-progress", ProgressPayload {
            percent: (*last_pct).max(90),
            message: "Converting to MP3...".to_string(),
            current_track: *current,
            total_tracks: *total,
            speed: speed.clone(),
        });
    }
}

/// Get the default download path (~/Music)
#[tauri::command]
fn get_download_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let music_path = format!("{}/Music", home);
    
    // Create directory if it doesn't exist
    let _ = fs::create_dir_all(&music_path);
    
    music_path
}

/// Download content from Spotify using spotdl
#[tauri::command]
async fn download_content(
    app: AppHandle,
    url: String,
    content_type: String,
    threads: u32,
    download_path: String,
    state: tauri::State<'_, DownloadState>,
) -> Result<String, String> {
    // Reset cancelled state
    state.is_cancelled.store(false, Ordering::SeqCst);
    
    // Ensure download directory exists
    let path = Path::new(&download_path);
    if !path.exists() {
        fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Get home directory for spotdl path
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let spotdl_path = format!("{}/.venv/bin/spotdl", home);

    // Emit starting progress
    let _ = app.emit("download-progress", ProgressPayload {
        percent: 5,
        message: "Starting download...".to_string(),
        current_track: 0,
        total_tracks: 0,
        speed: "".to_string(),
    });

    // Build spotdl command with full path
    let mut cmd = Command::new(&spotdl_path);
    cmd.arg("--format").arg("mp3");

    // Add threads for playlists and albums
    if content_type == "playlist" || content_type == "album" {
        cmd.arg("--threads").arg(threads.to_string());
    }

    cmd.arg(&url);
    cmd.current_dir(&download_path);
    
    // Capture stdout and stderr
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Spawn the process
    let mut child = cmd.spawn().map_err(|e| format!("Failed to run spotdl: {}", e))?;
    
    // Store the child PID
    let pid = child.id();
    state.child_pid.store(pid, Ordering::SeqCst);
    
    // Get stdout and stderr
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    
    // Shared state for tracking progress
    let current_track = Arc::new(Mutex::new(0u32));
    let total_tracks = Arc::new(Mutex::new(1u32));
    let last_percent = Arc::new(Mutex::new(5u32));
    let start_time = Instant::now();
    
    // Clone app handle and shared state for threads
    let app_stdout = app.clone();
    let current_stdout = Arc::clone(&current_track);
    let total_stdout = Arc::clone(&total_tracks);
    let last_pct_stdout = Arc::clone(&last_percent);
    let start_stdout = start_time.clone();
    
    let app_stderr = app.clone();
    let current_stderr = Arc::clone(&current_track);
    let total_stderr = Arc::clone(&total_tracks);
    let last_pct_stderr = Arc::clone(&last_percent);
    let start_stderr = start_time.clone();
    
    // Spawn threads to read stdout and stderr
    // When process is killed, pipes close and threads exit naturally
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                process_output_line(
                    &line,
                    &app_stdout,
                    &current_stdout,
                    &total_stdout,
                    &last_pct_stdout,
                    &start_stdout,
                );
            }
        }
    });
    
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                process_output_line(
                    &line,
                    &app_stderr,
                    &current_stderr,
                    &total_stderr,
                    &last_pct_stderr,
                    &start_stderr,
                );
            }
        }
    });
    
    // Wait for the process to complete
    let status = child.wait().map_err(|e| format!("Failed to wait for spotdl: {}", e))?;
    
    // Wait for reader threads to finish
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    
    // Clear the child PID
    state.child_pid.store(0, Ordering::SeqCst);
    
    // Check if cancelled
    if state.is_cancelled.load(Ordering::SeqCst) {
        let _ = app.emit("download-progress", ProgressPayload {
            percent: 0,
            message: "Download cancelled".to_string(),
            current_track: 0,
            total_tracks: 0,
            speed: "".to_string(),
        });
        return Err("Download cancelled by user".to_string());
    }
    
    // Calculate final speed (handle potential poisoned mutex)
    let final_current = current_track.lock().map(|c| *c).unwrap_or(0);
    let final_total = total_tracks.lock().map(|t| *t).unwrap_or(1);
    let elapsed_secs = start_time.elapsed().as_secs_f64();
    let final_speed = if final_current > 0 && elapsed_secs > 0.0 {
        let songs_per_min = (final_current as f64 / elapsed_secs) * 60.0;
        if songs_per_min >= 1.0 {
            format!("{:.1} songs/min", songs_per_min)
        } else {
            let secs_per_song = elapsed_secs / final_current as f64;
            format!("{:.0}s/song", secs_per_song)
        }
    } else {
        "".to_string()
    };

    if status.success() {
        let _ = app.emit("download-progress", ProgressPayload {
            percent: 100,
            message: "Download complete!".to_string(),
            current_track: final_total,
            total_tracks: final_total,
            speed: final_speed,
        });
        Ok(format!("{} downloaded successfully!", capitalize(&content_type)))
    } else {
        Err("Download failed. Please check the URL and try again.".to_string())
    }
}

fn extract_number(s: &str) -> Option<u32> {
    s.split_whitespace()
        .find_map(|word| word.parse::<u32>().ok())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(DownloadState::default())
        .invoke_handler(tauri::generate_handler![
            get_download_path,
            download_content,
            cancel_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
