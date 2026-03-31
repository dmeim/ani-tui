use tokio::process::Command;

use super::{PlayOptions, Player};

pub struct Vlc;

impl Player for Vlc {
    fn build_command(&self, opts: &PlayOptions) -> Command {
        let vlc_bin = if std::path::Path::new("/Applications/VLC.app/Contents/MacOS/VLC").exists() {
            "/Applications/VLC.app/Contents/MacOS/VLC"
        } else {
            "vlc"
        };

        let mut cmd = Command::new(vlc_bin);
        cmd.arg(&opts.url);
        cmd.arg("--play-and-exit");
        cmd.arg(format!("--meta-title={}", opts.title));

        if let Some(ref referer) = opts.referer {
            cmd.arg(format!("--http-referrer={referer}"));
        }
        if let Some(ref sub) = opts.subtitle_path {
            cmd.arg(format!("--sub-file={sub}"));
        }

        cmd
    }
}
