use tokio::process::Command;

use super::{PlayOptions, Player};

pub struct Iina;

impl Player for Iina {
    fn build_command(&self, opts: &PlayOptions) -> Command {
        let iina_bin = if std::path::Path::new("/Applications/IINA.app/Contents/MacOS/iina-cli").exists() {
            "/Applications/IINA.app/Contents/MacOS/iina-cli"
        } else {
            "iina"
        };

        let mut cmd = Command::new(iina_bin);
        cmd.arg(&opts.url);
        cmd.args(["--no-stdin", "--keep-running"]);
        cmd.arg(format!("--mpv-force-media-title={}", opts.title));

        if let Some(ref referer) = opts.referer {
            cmd.arg(format!("--mpv-referrer={referer}"));
        }
        if let Some(ref sub) = opts.subtitle_path {
            cmd.arg(format!("--mpv-sub-file={sub}"));
        }

        cmd
    }
}
