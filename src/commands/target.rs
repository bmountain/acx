use anyhow::{bail, Context, Result};
use std::{fs, path::Path};

pub fn resolve_problem_target(
    args: &[String],
    project_root: &Path,
    command_name: &str,
) -> Result<(String, String)> {
    match args {
        [] => infer_problem_target(project_root),
        [problem] => {
            let contest = infer_contest_for_problem(project_root, problem)?;
            Ok((contest, problem.to_string()))
        }
        [contest, problem] => Ok((contest.to_string(), problem.to_string())),
        _ => bail!("{command_name} accepts zero, one, or two positional arguments"),
    }
}

pub fn infer_problem_target(project_root: &Path) -> Result<(String, String)> {
    let (contest, problem) = infer_contest_and_optional_problem(project_root)?;
    let Some(problem) = problem else {
        bail!("failed to infer problem from current directory");
    };
    Ok((contest, problem))
}

pub fn infer_contest_target(project_root: &Path) -> Result<String> {
    let (contest, _) = infer_contest_and_optional_problem(project_root)?;
    Ok(contest)
}

fn infer_contest_and_optional_problem(project_root: &Path) -> Result<(String, Option<String>)> {
    let current = std::env::current_dir().context("failed to read current directory")?;
    let relative = current
        .strip_prefix(project_root)
        .unwrap_or(&current)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    let Some(index) = relative.iter().position(|part| part == "contests") else {
        bail!("failed to infer contest/problem from current directory");
    };
    let Some(contest) = relative.get(index + 1) else {
        bail!("failed to infer contest from current directory");
    };
    let problem = relative.get(index + 2).cloned();

    Ok((contest.to_string(), problem))
}

fn infer_contest_for_problem(project_root: &Path, problem: &str) -> Result<String> {
    if let Ok((contest, _)) = infer_problem_target(project_root) {
        return Ok(contest);
    }

    let contests_dir = project_root.join("contests");
    let mut matches = Vec::new();
    if contests_dir.exists() {
        for entry in fs::read_dir(&contests_dir)? {
            let contest_dir = entry?.path();
            if contest_dir.join(problem).is_dir() {
                if let Some(contest) = contest_dir.file_name().and_then(|name| name.to_str()) {
                    matches.push(contest.to_string());
                }
            }
        }
    }

    match matches.as_slice() {
        [contest] => Ok(contest.to_string()),
        [] => bail!("failed to infer contest for problem: {problem}"),
        _ => bail!("multiple contests contain problem {problem}; specify contest explicitly"),
    }
}
