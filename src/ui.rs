use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::{io::Write, time::Duration};

const LABEL_WIDTH: usize = 12;

pub fn action(label: &str, message: impl AsRef<str>) {
    println!(
        "{} {}",
        style(format_label(label)).cyan().bold(),
        message.as_ref()
    );
    flush_stdout();
}

pub fn success(label: &str, message: impl AsRef<str>) {
    println!(
        "{} {}",
        style(format_label(label)).green().bold(),
        message.as_ref()
    );
    flush_stdout();
}

pub fn warn(message: impl AsRef<str>) {
    eprintln!(
        "{} {}",
        style(format_label("Warning")).yellow().bold(),
        message.as_ref()
    );
}

pub fn error(message: impl AsRef<str>) {
    eprintln!(
        "{} {}",
        style(format_label("Error")).red().bold(),
        message.as_ref()
    );
}

pub fn caution(message: impl AsRef<str>) {
    eprintln!(
        "{} {}",
        style(format_label("Warning")).yellow().bold(),
        message.as_ref()
    );
}

pub fn note(label: &str, message: impl AsRef<str>) {
    println!("{} {}", style(format_label(label)).dim(), message.as_ref());
    flush_stdout();
}

fn format_label(label: &str) -> String {
    format!("{label:>LABEL_WIDTH$}")
}

fn flush_stdout() {
    let _ = std::io::stdout().flush();
}

pub struct DownloadProgress {
    bar: ProgressBar,
}

impl DownloadProgress {
    pub fn new(total: usize) -> Result<Self> {
        let bar = ProgressBar::new(total as u64);
        bar.set_draw_target(ProgressDrawTarget::stderr_with_hz(12));
        bar.set_style(
            ProgressStyle::with_template("{msg}\n{bar:36.cyan/black} {pos}/{len}")
                .expect("progress template should be valid")
                .progress_chars("█▓░"),
        );
        bar.enable_steady_tick(Duration::from_millis(120));
        Ok(Self { bar })
    }

    pub fn status(&self, label: &str, message: impl AsRef<str>) {
        self.bar.set_message(format!(
            "{} {}",
            style(format_label(label)).cyan().bold(),
            message.as_ref()
        ));
    }

    pub fn success(&self, label: &str, message: impl AsRef<str>) {
        self.bar.set_message(format!(
            "{} {}",
            style(format_label(label)).green().bold(),
            message.as_ref()
        ));
    }

    pub fn warning(&self, message: impl AsRef<str>) {
        self.bar.set_message(format!(
            "{} {}",
            style(format_label("Warning")).yellow().bold(),
            message.as_ref()
        ));
    }

    pub fn inc(&self) {
        self.bar.inc(1);
    }

    pub fn finish(&self, message: impl AsRef<str>) {
        self.bar.finish_with_message(format!(
            "{} {}",
            style(format_label("Finished")).green().bold(),
            message.as_ref()
        ));
    }
}
