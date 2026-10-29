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
            "{}/stackql_windows_{}.zip",
            STACKQL_RELEASE_BASE_URL,
            stackql_arch()
        )),
        Platform::Linux => Ok(format!(
            "{}/stackql_linux_{}.zip",
            STACKQL_RELEASE_BASE_URL,
            stackql_arch()
        )),
        Platform::Unknown => Err(AppError::CommandFailed(
            "Unsupported platform for stackql download".to_string(),
        )),
    }
}

/// Maps the compile-time CPU architecture to the suffix used by StackQL release
/// asset names (`arm64` or `amd64`). Decided at compile time so a native ARM64
/// build of stackql-deploy fetches a native ARM64 stackql rather than relying
/// on emulation.
fn stackql_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "amd64"
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
        .user_agent(concat!("stackql-deploy/", env!("CARGO_PKG_VERSION")))
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
            extract_zip(archive_path, dest_dir)?;

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

/// Extracts a zip archive into `dest_dir`.
///
/// Entries are written as plain files and directories only: no symlinks are
/// created and no permission bits from the archive are applied. An entry whose
/// path would escape `dest_dir` (absolute, or containing `..`) or that is a
/// symlink fails the whole extraction rather than being skipped, since the
/// stackql release archive never contains either.
fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<(), AppError> {
    let file = File::open(archive_path).map_err(AppError::IoError)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::CommandFailed(format!("Failed to open zip archive: {}", e)))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::CommandFailed(format!("Failed to extract file: {}", e)))?;

        let entry_name = String::from_utf8_lossy(entry.name_raw()).into_owned();
        let relative = entry.enclosed_name().ok_or_else(|| {
            AppError::CommandFailed(format!(
                "Refusing to extract zip entry with unsafe path: {}",
                entry_name
            ))
        })?;
        if entry.is_symlink() {
            return Err(AppError::CommandFailed(format!(
                "Refusing to extract symlink zip entry: {}",
                entry_name
            )));
        }
        let outpath = dest_dir.join(relative);

        if entry.is_dir() {
            fs::create_dir_all(&outpath).map_err(AppError::IoError)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(AppError::IoError)?;
            }
            let mut outfile = File::create(&outpath).map_err(AppError::IoError)?;
            io::copy(&mut entry, &mut outfile).map_err(AppError::IoError)?;
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    enum Entry<'a> {
        File(&'a str, &'a [u8]),
        Dir(&'a str),
        Symlink(&'a str, &'a str),
    }

    fn write_zip(path: &Path, entries: &[Entry]) {
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for entry in entries {
            match entry {
                Entry::File(name, data) => {
                    zip.start_file(*name, options).unwrap();
                    zip.write_all(data).unwrap();
                }
                Entry::Dir(name) => zip.add_directory(*name, options).unwrap(),
                Entry::Symlink(name, target) => zip.add_symlink(*name, *target, options).unwrap(),
            }
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extract_zip_writes_files_and_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("ok.zip");
        let dest = tmp.path().join("out");
        fs::create_dir(&dest).unwrap();
        write_zip(
            &archive,
            &[
                Entry::File("stackql", b"binary"),
                Entry::Dir("docs/"),
                Entry::File("docs/readme.txt", b"hello"),
            ],
        );

        extract_zip(&archive, &dest).unwrap();

        assert_eq!(fs::read(dest.join("stackql")).unwrap(), b"binary");
        assert!(dest.join("docs").is_dir());
        assert_eq!(fs::read(dest.join("docs/readme.txt")).unwrap(), b"hello");
    }

    #[test]
    fn extract_zip_rejects_path_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("traversal.zip");
        let dest = tmp.path().join("out");
        fs::create_dir(&dest).unwrap();
        write_zip(&archive, &[Entry::File("../escaped.txt", b"pwned")]);

        let err = extract_zip(&archive, &dest).unwrap_err();

        assert!(err.to_string().contains("unsafe path"), "{}", err);
        assert!(!tmp.path().join("escaped.txt").exists());
    }

    #[test]
    fn extract_zip_rejects_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("symlink.zip");
        let dest = tmp.path().join("out");
        fs::create_dir(&dest).unwrap();
        write_zip(&archive, &[Entry::Symlink("stackql", "/etc/passwd")]);

        let err = extract_zip(&archive, &dest).unwrap_err();

        assert!(err.to_string().contains("symlink"), "{}", err);
        assert!(fs::symlink_metadata(dest.join("stackql")).is_err());
    }
}
