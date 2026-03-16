// utils/download.rs

//! # Download Utility Module
//!
//! This module provides functions for downloading, extracting, and setting up the StackQL binary.
//! It supports various platforms including Linux, Windows, and macOS, handling differences in
//! extraction methods and permissions.
//!
//! ## Features
//! - Downloads the StackQL binary from a predefined URL.
//! - Supports progress tracking during download.
//! - Extracts the binary on various platforms (Windows, Linux, macOS).
//! - Sets executable permissions on Unix-like systems.
//!
//! ## Example Usage
//! ```rust
//! use crate::utils::download::download_binary;
//!
//! match download_binary() {
//!     Ok(path) => println!("Binary downloaded to: {}", path.display()),
//!     Err(e) => eprintln!("Failed to download binary: {}", e),
//! }
//! ```

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use reqwest::blocking::Client;
use zip::ZipArchive;

use crate::app::STACKQL_RELEASE_BASE_URL;
use crate::error::AppError;
use crate::utils::platform::{get_platform, Platform};

/// Retrieves the URL for downloading the StackQL binary based on OS and architecture.
pub fn get_download_url() -> Result<String, AppError> {
    let platform = get_platform();
    match platform {
        Platform::MacOS => Ok(format!(
            "{}/stackql_darwin_multiarch.pkg",
            STACKQL_RELEASE_BASE_URL
        )),
        Platform::Windows => Ok(format!(
            "{}/stackql_windows_amd64.zip",
            STACKQL_RELEASE_BASE_URL
        )),
        Platform::Linux => {
            let arch = if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "amd64"
            };
            Ok(format!(
                "{}/stackql_linux_{}.zip",
                STACKQL_RELEASE_BASE_URL, arch
            ))
        }
        Platform::Unknown => Err(AppError::CommandFailed(
            "Unsupported platform for stackql download".to_string(),
        )),
    }
}

/// Downloads the StackQL binary and extracts it to the current directory.
///
/// This function downloads the StackQL binary from a URL and unzips it if necessary.
/// It also sets executable permissions on Unix-like systems.
pub fn download_binary() -> Result<PathBuf, AppError> {
    let download_url = get_download_url()?;
    let current_dir = std::env::current_dir().map_err(AppError::IoError)?;
    let binary_name = crate::utils::platform::get_binary_name();
    let archive_name = Path::new(&download_url)
        .file_name()
        .ok_or_else(|| AppError::CommandFailed("Invalid URL".to_string()))?
        .to_string_lossy()
        .to_string();
    let archive_path = current_dir.join(&archive_name);

    // Download the file with progress bar
    debug!("Downloading from {}", download_url);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| AppError::CommandFailed(format!("Failed to create HTTP client: {}", e)))?;
    let mut response = client
        .get(&download_url)
        .send()
        .map_err(|e| AppError::CommandFailed(format!("Failed to download: {}", e)))?;

    let total_size = response.content_length().unwrap_or(0);
    let progress_bar = ProgressBar::new(total_size);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));

    let mut file = File::create(&archive_path).map_err(AppError::IoError)?;
    let mut buffer = [0u8; 8192];
    let mut downloaded: u64 = 0;
    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|e| AppError::CommandFailed(format!("Failed to read response: {}", e)))?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .map_err(AppError::IoError)?;
        downloaded += bytes_read as u64;
        progress_bar.set_position(downloaded);
    }
    progress_bar.finish_with_message("Download complete");

    // Extract the file based on platform
    debug!("Extracting the binary...");
    let binary_path = extract_binary(&archive_path, &current_dir, &binary_name)?;

    // Clean up the archive
    fs::remove_file(&archive_path).ok();

    // Set executable permissions on Unix-like systems
    if get_platform() != Platform::Windows {
        Command::new("chmod")
            .arg("+x")
            .arg(&binary_path)
            .output()
            .map_err(|e| {
                AppError::CommandFailed(format!("Failed to set executable permission: {}", e))
            })?;
    }

    debug!(
        "StackQL executable successfully installed at: {}",
        binary_path.display()
    );
    Ok(binary_path)
}

/// Extracts the StackQL binary from an archive.
fn extract_binary(
    archive_path: &Path,
    dest_dir: &Path,
    binary_name: &str,
) -> Result<PathBuf, AppError> {
    let binary_path = dest_dir.join(binary_name);

    match get_platform() {
        Platform::MacOS => {
            // For macOS, we need to use pkgutil
            // pkgutil --expand-full requires the destination directory to NOT exist
            let unpacked_dir = dest_dir.join("stackql_unpacked");
            if unpacked_dir.exists() {
                fs::remove_dir_all(&unpacked_dir).map_err(AppError::IoError)?;
            }

            let output = Command::new("pkgutil")
                .arg("--expand-full")
                .arg(archive_path)
                .arg(&unpacked_dir)
                .output()
                .map_err(|e| AppError::CommandFailed(format!("Failed to extract pkg: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(AppError::CommandFailed(format!(
                    "pkgutil failed: {}",
                    stderr
                )));
            }

            // Search for the stackql binary in the expanded pkg
            // The structure can vary: payload/usr/local/bin/stackql or
            // <subpkg>.pkg/payload/usr/local/bin/stackql
            let extracted_binary =
                find_file_recursive(&unpacked_dir, "stackql").ok_or_else(|| {
                    AppError::CommandFailed(
                        "Could not find stackql binary in extracted pkg".to_string(),
                    )
                })?;

            fs::copy(&extracted_binary, &binary_path).map_err(AppError::IoError)?;

            // Clean up
            fs::remove_dir_all(unpacked_dir).ok();
        }
        _ => {
            // For Windows and Linux, we use the zip file
            let file = File::open(archive_path).map_err(AppError::IoError)?;
            let mut archive = ZipArchive::new(file).map_err(|e| {
                AppError::CommandFailed(format!("Failed to open zip archive: {}", e))
            })?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i).map_err(|e| {
                    AppError::CommandFailed(format!("Failed to extract file: {}", e))
                })?;

                let outpath = match file.enclosed_name() {
                    Some(path) => dest_dir.join(path),
                    _none => continue,
                };

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath).map_err(AppError::IoError)?;
                } else {
                    let mut outfile = File::create(&outpath).map_err(AppError::IoError)?;
                    io::copy(&mut file, &mut outfile).map_err(AppError::IoError)?;
                }
            }

            // Check if we need to rename the binary on Windows
            if get_platform() == Platform::Windows {
                let potential_binary = dest_dir.join("stackql");
                if potential_binary.exists() && !binary_path.exists() {
                    fs::rename(potential_binary, &binary_path).map_err(AppError::IoError)?;
                }
            }
        }
    }

    if !binary_path.exists() {
        return Err(AppError::CommandFailed(format!(
            "Binary {} not found after extraction",
            binary_name
        )));
    }

    Ok(binary_path)
}

/// Recursively search for a file by name in a directory tree.
/// Returns the first match that is a regular file (not a directory).
fn find_file_recursive(dir: &Path, target_name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, target_name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(target_name) {
            return Some(path);
        }
    }
    None
}
