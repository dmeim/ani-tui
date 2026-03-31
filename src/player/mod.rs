pub mod iina;
pub mod mpv;
pub mod vlc;

use color_eyre::Result;
use tokio::process::Command;

use crate::config::PlayerName;

/// Options passed to the player when launching.
pub struct PlayOptions {
    pub url: String,
    pub title: String,
    pub referer: Option<String>,
    pub subtitle_path: Option<String>,
}

/// Trait for video player launchers.
pub trait Player {
    fn build_command(&self, opts: &PlayOptions) -> Command;
}

/// Launch the configured player with the given options.
pub async fn launch(player_name: PlayerName, custom_command: Option<&str>, opts: PlayOptions) -> Result<()> {
    let mut cmd = match player_name {
        PlayerName::Mpv => mpv::Mpv.build_command(&opts),
        PlayerName::Iina => iina::Iina.build_command(&opts),
        PlayerName::Vlc => vlc::Vlc.build_command(&opts),
        PlayerName::Quicktime => build_quicktime_command(&opts),
        PlayerName::Custom => {
            let bin = custom_command
                .ok_or_else(|| color_eyre::eyre::eyre!("Custom player set but no command configured"))?;
            let mut cmd = Command::new(bin);
            cmd.arg(&opts.url);
            cmd
        }
    };

    cmd.spawn()?;
    Ok(())
}

fn build_quicktime_command(opts: &PlayOptions) -> Command {
    let mut cmd = Command::new("open");
    cmd.args(["-a", "QuickTime Player", &opts.url]);
    cmd
}

/// Detect which players are installed on this system.
pub fn detect_installed() -> Vec<PlayerName> {
    let mut found = Vec::new();

    if which("mpv") { found.push(PlayerName::Mpv); }
    if which_iina() { found.push(PlayerName::Iina); }
    if which("vlc") || which_app("VLC") { found.push(PlayerName::Vlc); }
    if which_app("QuickTime Player") { found.push(PlayerName::Quicktime); }

    found
}

fn which(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn which_iina() -> bool {
    std::path::Path::new("/Applications/IINA.app").exists() || which("iina")
}

fn which_app(name: &str) -> bool {
    std::path::Path::new(&format!("/Applications/{name}.app")).exists()
}

/// Validate that the configured player can actually be found.
pub fn validate_player(name: PlayerName, custom_command: Option<&str>) -> bool {
    match name {
        PlayerName::Mpv => which("mpv"),
        PlayerName::Iina => which_iina(),
        PlayerName::Vlc => which("vlc") || which_app("VLC"),
        PlayerName::Quicktime => which_app("QuickTime Player"),
        PlayerName::Custom => custom_command.is_some_and(which),
    }
}
