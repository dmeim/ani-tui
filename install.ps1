$ErrorActionPreference = "Stop"

$Repo = "dmeim/ani-tui"
$BinaryName = "ani-tui.exe"
$Target = "x86_64-pc-windows-msvc"
$InstallDir = Join-Path $env:LOCALAPPDATA "ani-tui\bin"

Write-Host "Detected platform: $Target"

# Fetch latest release info from GitHub
Write-Host "Fetching latest release..."
$ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
$Release = Invoke-RestMethod -Uri $ReleaseUrl -Headers @{ "User-Agent" = "ani-tui-installer" }

$Asset = $Release.assets | Where-Object { $_.name -like "*$Target*" } | Select-Object -First 1
if (-not $Asset) {
    Write-Error "Could not find a release for $Target. Check https://github.com/$Repo/releases"
    exit 1
}

# Download and extract
$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "ani-tui-install"
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

$ZipPath = Join-Path $TmpDir "ani-tui.zip"
Write-Host "Downloading $($Asset.browser_download_url)..."
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $ZipPath

Write-Host "Extracting..."
Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

# Install
Write-Host "Installing to $InstallDir\$BinaryName..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $TmpDir $BinaryName) (Join-Path $InstallDir $BinaryName) -Force

# Clean up
Remove-Item -Recurse -Force $TmpDir

# Add to PATH if not already present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your user PATH. Restart your terminal for it to take effect."
}

Write-Host ""
Write-Host "ani-tui installed successfully!"
Write-Host ""
Write-Host "Usage:"
Write-Host "  ani-tui              Launch the TUI"
Write-Host "  ani-tui --update     Download and install the latest release"
Write-Host "  ani-tui --uninstall  Remove ani-tui from your system"
Write-Host "  ani-tui --version    Show version"
