$ErrorActionPreference = "Stop"

$BinaryName = "ani-tui.exe"
$RepoDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$DataDir = Join-Path $env:APPDATA "ani-tui"
$InstallDir = Join-Path $env:LOCALAPPDATA "ani-tui\bin"

Write-Host "Building ani-tui (release)..."
cargo build --release --manifest-path (Join-Path $RepoDir "Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

Write-Host "Installing to $InstallDir\$BinaryName..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $RepoDir "target\release\$BinaryName") (Join-Path $InstallDir $BinaryName) -Force

# Store repo path so --update knows where to rebuild from
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
Set-Content -Path (Join-Path $DataDir ".repo-path") -Value $RepoDir

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
Write-Host "  ani-tui --update     Pull latest changes and reinstall"
Write-Host "  ani-tui --uninstall  Remove ani-tui from your system"
Write-Host "  ani-tui --version    Show version"
