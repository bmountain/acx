use crate::project_config::BuildProfile;
use anyhow::{bail, Context, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub struct BuildPlan {
    pub source: PathBuf,
    pub run_command: String,
    build_command: String,
}

impl BuildPlan {
    pub fn new(problem_dir: &Path, profile: &BuildProfile) -> Self {
        let source = problem_dir.join(&profile.source);
        let binary = problem_dir.join(&profile.binary);
        let source_arg = shell_arg(&source);
        let binary_arg = shell_arg(&binary);

        let build_command = expand_command(&profile.build, &source_arg, &binary_arg);
        let run_command = expand_command(&profile.run, &source_arg, &binary_arg);

        Self {
            source,
            run_command,
            build_command,
        }
    }

    pub fn build_command(&self) -> &str {
        &self.build_command
    }

    pub fn build(&self) -> Result<()> {
        if !self.source.exists() {
            bail!("source file does not exist: {}", self.source.display());
        }

        let status = Command::new("sh")
            .arg("-c")
            .arg(&self.build_command)
            .status()
            .with_context(|| format!("failed to start build command: {}", self.build_command))?;

        if !status.success() {
            bail!("build failed: {}", self.build_command);
        }

        Ok(())
    }
}

fn expand_command(template: &str, source: &str, binary: &str) -> String {
    template
        .replace("{source}", source)
        .replace("{binary}", binary)
}

fn shell_arg(path: &Path) -> String {
    let text = path.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}
