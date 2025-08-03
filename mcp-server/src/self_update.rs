//! Self-update functionality for axon-mcp
//! 
//! Provides the ability to update the binary to the latest version
//! by checking GitHub releases and replacing the running binary.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const GITHUB_REPO: &str = "janreges/axon-mcp";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check for updates and perform self-update if available
pub async fn self_update(current_version: &str) -> Result<()> {
    println!("Checking for updates...");
    
    // Get latest release info
    let latest_release = get_latest_release().await?;
    let latest_version = latest_release.tag_name.trim_start_matches('v');
    
    // Compare versions
    if current_version == latest_version {
        println!("You are already running the latest version ({})", current_version);
        return Ok(());
    }
    
    println!("New version available: {} -> {}", current_version, latest_version);
    
    // Detect platform
    let platform = detect_platform()?;
    
    // Find matching asset
    let asset = find_matching_asset(&latest_release, &platform)
        .ok_or_else(|| anyhow::anyhow!("No release found for platform: {}", platform))?;
    
    println!("Downloading update from: {}", asset.browser_download_url);
    
    // Download and replace binary
    perform_update(&asset.browser_download_url).await?;
    
    println!("Update complete! Please restart axon-mcp to use the new version.");
    Ok(())
}

/// Get the latest release information from GitHub
async fn get_latest_release() -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    
    let client = reqwest::Client::builder()
        .user_agent("axon-mcp-updater")
        .build()?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch release information")?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("GitHub API returned status: {}", response.status()));
    }
    
    let release: GitHubRelease = response
        .json()
        .await
        .context("Failed to parse release JSON")?;
    
    Ok(release)
}

/// Detect the current platform
fn detect_platform() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    
    let platform = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("darwin", _) => "universal-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return Err(anyhow::anyhow!("Unsupported platform: {}-{}", os, arch)),
    };
    
    Ok(platform.to_string())
}

/// Find the asset matching our platform
fn find_matching_asset<'a>(release: &'a GitHubRelease, platform: &str) -> Option<&'a GitHubAsset> {
    release.assets.iter().find(|asset| {
        asset.name.contains(platform) && 
        (asset.name.ends_with(".tar.gz") || asset.name.ends_with(".zip"))
    })
}

/// Download and install the update
async fn perform_update(download_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    
    // Download to temp file
    let temp_dir = std::env::temp_dir();
    let temp_file = if download_url.ends_with(".zip") {
        temp_dir.join("axon-mcp-update.zip")
    } else {
        temp_dir.join("axon-mcp-update.tar.gz")
    };
    
    let response = client
        .get(download_url)
        .send()
        .await
        .context("Failed to download update")?;
    
    let bytes = response
        .bytes()
        .await
        .context("Failed to read update bytes")?;
    
    fs::write(&temp_file, bytes)
        .context("Failed to write update to temp file")?;
    
    // Extract the binary
    let extracted_binary = extract_binary(&temp_file)?;
    
    // Get current executable path
    let current_exe = std::env::current_exe()
        .context("Failed to get current executable path")?;
    
    // Replace the binary (platform-specific)
    replace_binary(&extracted_binary, &current_exe)?;
    
    // Clean up
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&extracted_binary);
    
    Ok(())
}

/// Extract binary from archive
fn extract_binary(archive_path: &Path) -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let extract_dir = temp_dir.join("axon-mcp-extract");
    
    // Create extraction directory
    fs::create_dir_all(&extract_dir)?;
    
    if archive_path.extension().and_then(|s| s.to_str()) == Some("zip") {
        // Windows ZIP extraction
        #[cfg(windows)]
        {
            use zip::ZipArchive;
            let file = fs::File::open(archive_path)?;
            let mut archive = ZipArchive::new(file)?;
            archive.extract(&extract_dir)?;
        }
        
        #[cfg(not(windows))]
        return Err(anyhow::anyhow!("ZIP extraction not supported on this platform"));
    } else {
        // Unix tar.gz extraction
        Command::new("tar")
            .args(&["-xzf", archive_path.to_str().unwrap()])
            .current_dir(&extract_dir)
            .output()
            .context("Failed to extract tar.gz")?;
    }
    
    // Find the binary
    let binary_name = if cfg!(windows) { "axon-mcp.exe" } else { "axon-mcp" };
    let binary_path = extract_dir.join(binary_name);
    
    if !binary_path.exists() {
        return Err(anyhow::anyhow!("Binary not found in archive"));
    }
    
    Ok(binary_path)
}

/// Replace the current binary with the new one
fn replace_binary(new_binary: &Path, current_binary: &Path) -> Result<()> {
    // On Windows, we need to rename the current binary first
    #[cfg(windows)]
    {
        let backup_path = current_binary.with_extension("exe.old");
        fs::rename(current_binary, &backup_path)
            .context("Failed to backup current binary")?;
    }
    
    // Copy new binary with proper permissions
    fs::copy(new_binary, current_binary)
        .context("Failed to copy new binary")?;
    
    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(current_binary)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(current_binary, perms)?;
    }
    
    Ok(())
}

/// Version information for --version flag
pub fn print_version() {
    println!("axon-mcp {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_platform() {
        let platform = detect_platform();
        assert!(platform.is_ok());
        let platform = platform.unwrap();
        assert!(!platform.is_empty());
    }
}