# Axon MCP Installer for Windows
# https://github.com/janreges/axon-mcp
#
# This script installs axon-mcp on Windows systems.
# Run in PowerShell with: irm https://raw.githubusercontent.com/janreges/axon-mcp/main/install.ps1 | iex

[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\axon-mcp\bin"
)

$ErrorActionPreference = "Stop"

# Configuration
$GitHubRepo = "janreges/axon-mcp"
$BinaryName = "axon-mcp"

# Colors for output
$OriginalForegroundColor = $Host.UI.RawUI.ForegroundColor

function Write-Info {
    param([string]$Message)
    Write-Host "ℹ " -ForegroundColor Blue -NoNewline
    Write-Host " $Message"
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ " -ForegroundColor Green -NoNewline
    Write-Host " $Message"
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠ " -ForegroundColor Yellow -NoNewline
    Write-Host " $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ " -ForegroundColor Red -NoNewline
    Write-Host " $Message" -ForegroundColor Red
}

function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "x86" { return "x86" }
        "ARM64" { return "aarch64" }
        default { 
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

function Test-CommandExists {
    param([string]$Command)
    $null = Get-Command $Command -ErrorAction SilentlyContinue
    return $?
}

function Add-ToPath {
    param([string]$Directory)
    
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    
    if ($currentPath -notlike "*$Directory*") {
        Write-Info "Adding $Directory to user PATH..."
        
        $newPath = "$currentPath;$Directory"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        
        # Update current session
        $env:Path = "$env:Path;$Directory"
        
        Write-Success "Added to PATH successfully"
        Write-Warning "You may need to restart your terminal for PATH changes to take effect"
        return $true
    }
    
    return $false
}

function Install-Binary {
    Write-Host ""
    Write-Host "Axon MCP Installer for Windows" -ForegroundColor Cyan
    Write-Host "==============================" -ForegroundColor Cyan
    Write-Host ""
    
    # Check if running as administrator
    if (Test-Administrator) {
        Write-Warning "Running as Administrator. Installing to user directory anyway."
    }
    
    # Detect architecture
    $arch = Get-Architecture
    $platform = "$arch-pc-windows-msvc"
    Write-Info "Detected platform: $platform"
    
    # Construct download URL
    $assetName = "$BinaryName-$platform.zip"
    if ($Version -eq "latest") {
        $downloadUrl = "https://github.com/$GitHubRepo/releases/latest/download/$assetName"
    } else {
        $downloadUrl = "https://github.com/$GitHubRepo/releases/download/$Version/$assetName"
    }
    
    Write-Info "Download URL: $downloadUrl"
    
    # Create install directory
    if (!(Test-Path $InstallDir)) {
        Write-Info "Creating installation directory: $InstallDir"
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    
    # Download binary
    $tempFile = Join-Path $env:TEMP "$BinaryName-$([System.Guid]::NewGuid()).zip"
    
    try {
        Write-Info "Downloading $BinaryName..."
        $ProgressPreference = 'SilentlyContinue'
        
        # Download with timeout
        $webClient = New-Object System.Net.WebClient
        $webClient.Headers.Add("User-Agent", "axon-mcp-installer")
        
        try {
            $downloadTask = $webClient.DownloadFileTaskAsync($downloadUrl, $tempFile)
            $timeoutTask = [System.Threading.Tasks.Task]::Delay(300000) # 5 minutes timeout
            
            $completedTask = [System.Threading.Tasks.Task]::WhenAny($downloadTask, $timeoutTask).Result
            
            if ($completedTask -eq $timeoutTask) {
                $webClient.CancelAsync()
                throw "Download timeout after 5 minutes"
            }
            
            $downloadTask.Wait()
        } finally {
            $webClient.Dispose()
        }
        
        $ProgressPreference = 'Continue'
        
        # Verify download
        if (!(Test-Path $tempFile) -or (Get-Item $tempFile).Length -eq 0) {
            throw "Downloaded file is empty or missing"
        }
        
        # Extract zip to temp location first
        Write-Info "Extracting binary..."
        $tempExtractDir = Join-Path $env:TEMP "axon-extract-$([System.Guid]::NewGuid())"
        New-Item -ItemType Directory -Path $tempExtractDir -Force | Out-Null
        
        try {
            Expand-Archive -Path $tempFile -DestinationPath $tempExtractDir -Force
            
            # Find the binary
            $extractedBinary = Get-ChildItem -Path $tempExtractDir -Filter "$BinaryName.exe" -Recurse | Select-Object -First 1
            if (!$extractedBinary) {
                throw "Binary not found in archive"
            }
            
            # Move atomically
            $binaryPath = Join-Path $InstallDir "$BinaryName.exe"
            $tempBinary = Join-Path $InstallDir "$BinaryName.tmp.exe"
            Move-Item -Path $extractedBinary.FullName -Destination $tempBinary -Force
            Move-Item -Path $tempBinary -Destination $binaryPath -Force
        } finally {
            if (Test-Path $tempExtractDir) {
                Remove-Item $tempExtractDir -Recurse -Force
            }
        }
        
        # Verify binary exists
        $binaryPath = Join-Path $InstallDir "$BinaryName.exe"
        if (!(Test-Path $binaryPath)) {
            throw "Binary not found after extraction"
        }
        
        Write-Success "Binary installed to: $binaryPath"
        
    } catch {
        Write-Error "Failed to download or extract binary: $_"
        exit 1
    } finally {
        # Clean up temp file
        if (Test-Path $tempFile) {
            Remove-Item $tempFile -Force
        }
    }
    
    # Add to PATH if needed
    Add-ToPath $InstallDir | Out-Null
    
    # Configure Claude Code
    Configure-ClaudeCode -BinaryPath $binaryPath
    
    # Health check
    Write-Info "Running health check..."
    try {
        $version = & $binaryPath --version 2>&1
        Write-Success "axon-mcp is installed and working: $version"
    } catch {
        Write-Warning "Could not verify installation. Error: $_"
    }
    
    Write-Host ""
    Write-Success "Installation complete!"
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "  1. Restart your terminal to ensure PATH is updated"
    Write-Host "  2. Verify installation: " -NoNewline
    Write-Host "$BinaryName --version" -ForegroundColor Yellow
    Write-Host "  3. In Claude Code, verify connection with: " -NoNewline
    Write-Host "/mcp" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "For updates, run: " -NoNewline
    Write-Host "$BinaryName self-update" -ForegroundColor Yellow
    Write-Host ""
}

function Configure-ClaudeCode {
    param([string]$BinaryPath)
    
    Write-Info "Configuring Claude Code..."
    
    # Check if claude CLI exists
    if (Test-CommandExists "claude") {
        Write-Info "Found claude CLI, attempting automatic configuration..."
        
        try {
            # Try claude mcp add
            & claude mcp add $BinaryName -- $BinaryPath 2>$null
            if ($LASTEXITCODE -eq 0) {
                Write-Success "Claude Code configured successfully!"
                return
            }
        } catch {
            Write-Warning "claude mcp add failed, trying alternative method..."
        }
        
        # Try claude mcp add-json
        try {
            $jsonConfig = @{
                command = @($BinaryPath)
            } | ConvertTo-Json -Compress
            
            $jsonConfig | & claude mcp add-json $BinaryName 2>$null
            if ($LASTEXITCODE -eq 0) {
                Write-Success "Claude Code configured successfully using add-json!"
                return
            }
        } catch {
            # Continue to manual instructions
        }
    }
    
    # Manual configuration instructions
    Write-Warning "Could not configure Claude Code automatically."
    Write-Host ""
    Write-Host "Please add the following configuration manually:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Configuration file location:" -ForegroundColor Cyan
    Write-Host "  $env:APPDATA\Claude\claude_desktop_config.json"
    Write-Host ""
    Write-Host "Add this to the mcpServers section:" -ForegroundColor Cyan
    Write-Host @"
{
  "mcpServers": {
    "$BinaryName": {
      "command": ["$($BinaryPath.Replace('\', '\\'))"]
    }
  }
}
"@ -ForegroundColor Blue
    Write-Host ""
}

# Main execution
try {
    Install-Binary
} catch {
    Write-Error "Installation failed: $_"
    exit 1
} finally {
    $Host.UI.RawUI.ForegroundColor = $OriginalForegroundColor
}