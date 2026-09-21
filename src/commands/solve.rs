use crate::commands::target::resolve_problem_target;
use crate::models::ProblemMetadata;
use crate::project_config::ProjectConfig;
use crate::{fs_layout, ui};
use anyhow::{bail, Context, Result};
use std::{fs, process::Command};

pub fn solve(args: &[String]) -> Result<()> {
    let (config, project_root) = ProjectConfig::load_from_current_dir()?;
    let (contest, problem) =
        resolve_problem_target(args, &project_root, &config.contests_dir, "solve")?;
    let (_, profile) = config.profile(None)?;
    let problem_dir = project_root.join(fs_layout::problem_dir(
        &config.contests_dir,
        &contest,
        &problem,
    ));
    let metadata = read_problem_metadata(&problem_dir)?;
    let statement = problem_dir.join(
        metadata
            .as_ref()
            .map_or("statement.md", |m| m.statement.as_str()),
    );
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

fn read_problem_metadata(problem_dir: &std::path::Path) -> Result<Option<ProblemMetadata>> {
    let path = problem_dir.join("metadata.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read metadata: {}", path.display()))?;
    let metadata = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse metadata: {}", path.display()))?;
    Ok(Some(metadata))
}
