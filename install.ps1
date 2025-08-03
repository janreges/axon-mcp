# Axon MCP Installer for Windows
# https://github.com/janreges/axon-mcp
#
# This script installs axon-mcp on Windows systems.
# Run in PowerShell with: irm https://raw.githubusercontent.com/janreges/axon-mcp/main/install.ps1 | iex

[CmdletBinding(SupportsShouldProcess)]
param(
    [string]$Version = "latest",
    [string]$InstallDir = "",
    [switch]$ClaudeCodeProject,
    [switch]$ClaudeCodeUser
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

function Write-ErrorAndExit {
    param([string]$Message)
    Write-Host "✗ " -ForegroundColor Red -NoNewline
    Write-Host " $Message" -ForegroundColor Red
    exit 1
}

function Prompt-YesNo {
    param(
        [string]$Message,
        [ValidateSet('Y', 'N')][string]$Default = 'Y'
    )
    while ($true) {
        $promptText = if ($Default -eq 'Y') { "$Message [Y/n]: " } else { "$Message [y/N]: " }
        $response = Read-Host -Prompt $promptText
        if ([string]::IsNullOrWhiteSpace($response)) {
            if ($Default -eq 'Y') { return $true } else { return $false }
        }
        switch ($response.ToLower()) {
            "y" { return $true }
            "n" { return $false }
            default { Write-Warning "Please answer 'y' or 'n'." }
        }
    }
}

function Find-ProjectRoot {
    $currentDir = Get-Location
    $rootFound = $null

    while ($currentDir.Path -ne (Get-PSDrive -Name $currentDir.PSDrive.Name).Root -and $currentDir.Path -ne "") {
        if (Test-Path (Join-Path $currentDir ".git") -PathType Container) {
            $rootFound = $currentDir.Path
            break
        } elseif (Test-Path (Join-Path $currentDir ".claude") -PathType Container) {
            $rootFound = $currentDir.Path
            break
        }
        $currentDir = (Get-Item $currentDir).Parent
    }

    return $rootFound
}

function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "amd64" }
        "x86" { return "x86" }
        "ARM64" { return "arm64" }
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
    
    # --- CLI Parsing and Installation Path Logic ---
    $ProjectRoot = $null
    $InstallMode = "auto" # 'auto', 'project', 'user'
    $TargetInstallDir = ""
    
    # Determine installation mode based on CLI switches
    if ($ClaudeCodeProject.IsPresent -and $ClaudeCodeUser.IsPresent) {
        Write-ErrorAndExit "Cannot use -ClaudeCodeProject and -ClaudeCodeUser simultaneously."
    } elseif ($ClaudeCodeProject.IsPresent) {
        $InstallMode = "project"
    } elseif ($ClaudeCodeUser.IsPresent) {
        $InstallMode = "user"
    }
    
    # Determine project root if not explicitly set by CLI
    if ($InstallMode -eq "auto" -or $InstallMode -eq "project") {
        Write-Info "Detecting project root..."
        $ProjectRoot = Find-ProjectRoot
        if ($ProjectRoot) {
            Write-Success "Project root found: $ProjectRoot"
            if ($InstallMode -eq "auto") {
                $InstallMode = "project" # Default to project scope if detected and no override
            }
        } else {
            Write-Warning "Project root not found. Installation will continue in user scope."
            if ($InstallMode -eq "project") {
                Write-ErrorAndExit "Argument -ClaudeCodeProject was provided, but project root was not found. Aborting."
            }
            $InstallMode = "user" # Fallback to user if -ClaudeCodeProject not specified
        }
    }
    
    # Set installation directory based on determined mode
    if ($InstallMode -eq "project") {
        $TargetInstallDir = Join-Path $ProjectRoot ".axon\bin"
        Write-Info "Project-scoped installation to: $TargetInstallDir"
    } elseif ($InstallMode -eq "user") {
        $TargetInstallDir = if ($InstallDir) { $InstallDir } else { "$env:LOCALAPPDATA\axon-mcp\bin" }
        Write-Info "User-scoped installation to: $TargetInstallDir"
    } else {
        Write-ErrorAndExit "Unknown installation mode: $InstallMode"
    }
    
    # WhatIf support
    if ($WhatIfPreference) {
        Write-Host "WhatIf: Would install axon-mcp version $Version to $TargetInstallDir" -ForegroundColor Yellow
        return
    }
    
    # Check if running as administrator
    if (Test-Administrator) {
        Write-Warning "Running as Administrator. Installing to user directory anyway."
    }
    
    # Detect architecture
    $arch = Get-Architecture
    $platform = "windows-$arch"
    Write-Info "Detected platform: $platform"
    
    # Get version for asset name
    if ($Version -eq "latest") {
        # Get latest version from GitHub API
        try {
            $apiResponse = Invoke-RestMethod -Uri "https://api.github.com/repos/$GitHubRepo/releases/latest" -ErrorAction Stop
            $versionTag = $apiResponse.tag_name
        } catch {
            Write-Error "Failed to get latest version from GitHub API: $_"
            exit 1
        }
    } else {
        $versionTag = "v$Version"
    }
    
    # Construct beautiful asset name: axon-mcp-{platform}-v{version}.zip
    $assetName = "$BinaryName-$platform-$versionTag.zip"
    
    # Construct download URL
    if ($Version -eq "latest") {
        $downloadUrl = "https://github.com/$GitHubRepo/releases/latest/download/$assetName"
    } else {
        $downloadUrl = "https://github.com/$GitHubRepo/releases/download/$versionTag/$assetName"
    }
    
    Write-Info "Download URL: $downloadUrl"
    
    # Create install directory
    if (!(Test-Path $TargetInstallDir -PathType Container)) {
        Write-Info "Creating installation directory: $TargetInstallDir"
        try {
            New-Item -Path $TargetInstallDir -ItemType Directory -Force | Out-Null
        } catch {
            Write-ErrorAndExit "Failed to create directory $TargetInstallDir. Check permissions. Error: $($_.Exception.Message)"
        }
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
            $binaryPath = Join-Path $TargetInstallDir "$BinaryName.exe"
            $tempBinary = Join-Path $TargetInstallDir "$BinaryName.tmp.exe"
            Move-Item -Path $extractedBinary.FullName -Destination $tempBinary -Force
            Move-Item -Path $tempBinary -Destination $binaryPath -Force
        } finally {
            if (Test-Path $tempExtractDir) {
                Remove-Item $tempExtractDir -Recurse -Force
            }
        }
        
        # Verify binary exists
        $binaryPath = Join-Path $TargetInstallDir "$BinaryName.exe"
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
    
    # Add to PATH if needed (only for user installs)
    if ($InstallMode -eq "user") {
        Add-ToPath $TargetInstallDir | Out-Null
    }
    
    # Health check
    Write-Info "Running health check..."
    try {
        $version = & $binaryPath --version 2>&1
        Write-Success "axon-mcp is installed and working: $version"
    } catch {
        Write-Warning "Could not verify installation. Error: $_"
    }
    
    # --- Post-Installation Automation ---
    if ($InstallMode -eq "project") {
        Write-Info "Running automation steps for project-scoped installation..."

        # Add .axon/ to .gitignore
        $GitignorePath = Join-Path $ProjectRoot ".gitignore"
        if (Test-Path $GitignorePath -PathType Leaf) {
            $gitignoreContent = Get-Content $GitignorePath -Raw
            if (-not ($gitignoreContent -match "(?m)^\.axon/$")) { # (?m) for multiline match
                if (Prompt-YesNo "Add '.axon/' to '$GitignorePath'?" "Y") {
                    Add-Content -Path $GitignorePath -Value "`n.axon/"
                    Write-Success "Added '.axon/' to '$GitignorePath'."
                } else {
                    Write-Info "Adding '.axon/' to .gitignore skipped."
                }
            } else {
                Write-Info "'.axon/' is already in '$GitignorePath'."
            }
        } else {
            Write-Info ".gitignore not found in '$ProjectRoot'. Skipping adding '.axon/'."
        }

        # claude mcp add
        $ClaudeDir = Join-Path $ProjectRoot ".claude"
        if (Test-Path $ClaudeDir -PathType Container) {
            Write-Info "Detected '.claude/' folder in project root."
            if (Prompt-YesNo "Run 'claude mcp add' for this project?" "Y") {
                Write-Info "Running 'claude mcp add'..."
                try {
                    Push-Location $ProjectRoot
                    claude mcp add axon-mcp -- $binaryPath | Out-Null
                    if ($LASTEXITCODE -eq 0) {
                        Write-Success "'claude mcp add' executed successfully."
                    } else {
                        Write-Warning "'claude mcp add' failed with code $LASTEXITCODE. Check output for details."
                    }
                } catch {
                    Write-Warning "Error running 'claude mcp add': $($_.Exception.Message)"
                } finally {
                    Pop-Location
                }
            } else {
                Write-Info "Running 'claude mcp add' skipped."
            }
        } else {
            Write-Info "'.claude/' folder not found in project root. Skipping 'claude mcp add'."
        }

        Write-Info "To use '$BinaryName' in this project, we recommend adding '$TargetInstallDir' to your PATH, e.g. in your PowerShell profile (`$PROFILE):"
        Write-Info "  `$env:Path += `";$TargetInstallDir`""
        Write-Info "Or run the binary directly: '$TargetInstallDir\$BinaryName.exe'."
        Write-Info "Or use alias: `Set-Alias -Name $BinaryName -Value '$TargetInstallDir\$BinaryName.exe'`"

    } elseif ($InstallMode -eq "user") {
        Write-Info "Running automation steps for user-scoped installation..."
        Write-Info "Make sure '$TargetInstallDir' is in your PATH. You can add it to your PowerShell profile (`$PROFILE):"
        Write-Info "  `$env:Path += `";$TargetInstallDir`""
        Write-Info "Then run '. `$PROFILE' or restart terminal."
    }
    
    Write-Host ""
    Write-Success "Installation complete!"
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Cyan
    if ($InstallMode -eq "project") {
        Write-Host "  1. Use: " -NoNewline
        Write-Host "$TargetInstallDir\$BinaryName.exe --version" -ForegroundColor Yellow
        Write-Host "  2. In Claude Code, verify connection with: " -NoNewline
        Write-Host "/mcp" -ForegroundColor Yellow
    } else {
        Write-Host "  1. Restart your terminal to ensure PATH is updated"
        Write-Host "  2. Verify installation: " -NoNewline
        Write-Host "$BinaryName --version" -ForegroundColor Yellow
        Write-Host "  3. In Claude Code, verify connection with: " -NoNewline
        Write-Host "/mcp" -ForegroundColor Yellow
    }
    Write-Host ""
    Write-Host "For updates, run: " -NoNewlines
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