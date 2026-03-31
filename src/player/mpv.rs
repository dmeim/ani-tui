use tokio::process::Command;

use super::{PlayOptions, Player};

pub struct Mpv;

impl Player for Mpv {
    fn build_command(&self, opts: &PlayOptions) -> Command {
        let mut cmd = Command::new("mpv");
        cmd.arg(&opts.url);
        cmd.arg(format!("--force-media-title={}", opts.title));

        if let Some(ref referer) = opts.referer {
            cmd.arg(format!("--referrer={referer}"));
        }
        if let Some(ref sub) = opts.subtitle_path {
            cmd.arg(format!("--sub-file={sub}"));
        }

        cmd
    }
}
