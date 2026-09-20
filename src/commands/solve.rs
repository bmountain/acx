use crate::commands::target::resolve_problem_target;
use crate::project_config::ProjectConfig;
use crate::{fs_layout, ui};
use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn solve(args: &[String]) -> Result<()> {
    let (config, project_root) = ProjectConfig::load_from_current_dir()?;
    let (contest, problem) = resolve_problem_target(args, &project_root, "solve")?;
    let (_, profile) = config.profile(None)?;
    let problem_dir = project_root.join(fs_layout::problem_dir(&contest, &problem));
    let statement = problem_dir.join("statement.md");
    let source = problem_dir.join(&profile.source);

    if !statement.exists() {
        bail!("statement file does not exist: {}", statement.display());
    }
    if !source.exists() {
        bail!("source file does not exist: {}", source.display());
    }

    let command = config.solve.command.clone();
    let command = command
        .replace("{statement}", &shell_arg(&statement))
        .replace("{source}", &shell_arg(&source));

    ui::info(format!("Opening {}", problem_dir.display()));
    let status = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .status()
        .with_context(|| format!("failed to start editor command: {command}"))?;
    if status.success() {
        Ok(())
    } else {
        bail!("editor command exited with {status}")
    }
}

fn shell_arg(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}
