use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::time::Duration;

pub fn info(message: impl AsRef<str>) {
    println!("{}", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    eprintln!(
        "{}",
        style(format!("warning: {}", message.as_ref())).yellow()
    );
}

pub fn success(message: impl AsRef<str>) {
    println!("{}", style(message.as_ref()).green());
}

pub fn caution(message: impl AsRef<str>) {
    eprintln!("{}", style(message.as_ref()).yellow());
}

pub struct DownloadProgress {
    bar: ProgressBar,
}

impl DownloadProgress {
    pub fn new(total: usize) -> Result<Self> {
        let bar = ProgressBar::new(total as u64);
        bar.set_draw_target(ProgressDrawTarget::stderr_with_hz(12));
        bar.set_style(
            ProgressStyle::with_template("{msg}\n{bar:32.cyan/black} {pos}/{len}")
                .expect("progress template should be valid")
                .progress_chars("█▓░"),
        );
        bar.enable_steady_tick(Duration::from_millis(120));
        Ok(Self { bar })
    }

    pub fn status(&self, message: impl Into<String>) {
        self.bar.set_message(message.into());
    }

    pub fn warning(&self, message: impl AsRef<str>) {
        self.bar.set_message(
            style(format!("warning: {}", message.as_ref()))
                .yellow()
                .to_string(),
        );
    }

    pub fn inc(&self) {
        self.bar.inc(1);
    }

    pub fn finish(&self, message: impl Into<String>) {
        self.bar.finish_with_message(message.into());
    }
}
