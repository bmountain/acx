use crate::models::ContestMetadata;
use anyhow::{bail, Context, Result};
use std::{fs, path::Path};

pub fn resolve_problem_target(
    args: &[String],
    project_root: &Path,
    contests_dir: &str,
    command_name: &str,
) -> Result<(String, String)> {
    match args {
        [] => infer_problem_target(project_root, contests_dir),
        [selector] => resolve_problem_selector(project_root, contests_dir, selector),
        [contest, problem] => Ok((
            normalize_contest_selector(contests_dir, contest),
            problem.to_string(),
        )),
        _ => bail!("{command_name} accepts zero, one, or two positional arguments"),
    }
}

pub fn resolve_contest_target(
    contest: Option<&str>,
    project_root: &Path,
    contests_dir: &str,
) -> Result<String> {
    match contest {
        Some(contest) => Ok(normalize_contest_selector(contests_dir, contest)),
        None => infer_contest_target(project_root, contests_dir),
    }
}

pub fn infer_problem_target(project_root: &Path, contests_dir: &str) -> Result<(String, String)> {
    let (contest, problem) = infer_contest_and_optional_problem(project_root, contests_dir)?;
    let Some(problem) = problem else {
        bail!("failed to infer problem from current directory");
    };
    Ok((contest, problem))
}

pub fn infer_contest_target(project_root: &Path, contests_dir: &str) -> Result<String> {
    if let Ok((contest, problem)) = infer_contest_and_optional_problem(project_root, contests_dir) {
        if problem.is_some() || contest_metadata_path(project_root, contests_dir, &contest).exists()
        {
            return Ok(contest);
        }
    }

    let contests_root = project_root.join(contests_dir);
    let contests = contest_metadata_entries(&contests_root)?;
    match contests.as_slice() {
        [contest] => Ok(contest.clone()),
        [] => bail!("failed to infer contest from current directory"),
        _ => bail!("multiple contests found; specify contest explicitly"),
    }
}

fn resolve_problem_selector(
    project_root: &Path,
    contests_dir: &str,
    selector: &str,
) -> Result<(String, String)> {
    if let Some((contest, problem)) = parse_problem_path(contests_dir, selector) {
        return Ok((contest, problem));
    }

    if let Ok((contest, _)) = infer_contest_and_optional_problem(project_root, contests_dir) {
        if problem_dir(project_root, contests_dir, &contest, selector).is_dir() {
            return Ok((contest, selector.to_string()));
        }
    }

    let matches = find_problem_in_metadata(project_root, contests_dir, selector)?;
    match matches.as_slice() {
        [(contest, problem)] => Ok((contest.clone(), problem.clone())),
        [] => bail!("failed to infer contest for problem: {selector}"),
        _ => bail!("multiple contests contain problem {selector}; specify contest explicitly"),
    }
}

fn infer_contest_and_optional_problem(
    project_root: &Path,
    contests_dir: &str,
) -> Result<(String, Option<String>)> {
    let current = std::env::current_dir().context("failed to read current directory")?;
    let contests_root = project_root.join(contests_dir);
    let relative = current
        .strip_prefix(&contests_root)
        .with_context(|| "failed to infer contest/problem from current directory")?;
    let parts = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    let Some(contest) = parts.first() else {
        bail!("failed to infer contest from current directory");
    };
    let problem = parts.get(1).cloned();
    Ok((contest.to_string(), problem))
}

fn parse_problem_path(contests_dir: &str, selector: &str) -> Option<(String, String)> {
    let parts = selector
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    let parts = if parts.first().copied() == Some(contests_dir) {
        &parts[1..]
    } else {
        parts.as_slice()
    };

    match parts {
        [contest, problem] => Some((contest.to_string(), problem.to_string())),
        [.., contest, problem] => Some((contest.to_string(), problem.to_string())),
        _ => None,
    }
}

fn normalize_contest_selector(contests_dir: &str, selector: &str) -> String {
    let parts = selector
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    let parts = if parts.first().copied() == Some(contests_dir) {
        &parts[1..]
    } else {
        parts.as_slice()
    };
    parts.last().copied().unwrap_or(selector).to_string()
}

fn find_problem_in_metadata(
    project_root: &Path,
    contests_dir: &str,
    problem: &str,
) -> Result<Vec<(String, String)>> {
    let contests_root = project_root.join(contests_dir);
    let mut matches = Vec::new();
    for contest in contest_metadata_entries(&contests_root)? {
        let metadata = read_contest_metadata(project_root, contests_dir, &contest)?;
        for task in metadata.tasks {
            if task.id == problem || task.path == problem || task.task_screen_name == problem {
                matches.push((contest.clone(), task.id));
            }
        }
    }
    Ok(matches)
}

fn contest_metadata_entries(contests_root: &Path) -> Result<Vec<String>> {
    if !contests_root.exists() {
        return Ok(Vec::new());
    }

    let mut contests = Vec::new();
    for entry in fs::read_dir(contests_root)? {
        let path = entry?.path();
        if !path.join("metadata.json").is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            contests.push(name.to_string());
        }
    }
    contests.sort();
    Ok(contests)
}

fn read_contest_metadata(
    project_root: &Path,
    contests_dir: &str,
    contest: &str,
) -> Result<ContestMetadata> {
    let path = contest_metadata_path(project_root, contests_dir, contest);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read contest metadata: {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse contest metadata: {}", path.display()))
}

fn contest_metadata_path(
    project_root: &Path,
    contests_dir: &str,
    contest: &str,
) -> std::path::PathBuf {
    project_root
        .join(contests_dir)
        .join(contest)
        .join("metadata.json")
}

fn problem_dir(
    project_root: &Path,
    contests_dir: &str,
    contest: &str,
    problem: &str,
) -> std::path::PathBuf {
    project_root.join(contests_dir).join(contest).join(problem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("acx-target-test-{suffix}"));
        fs::create_dir_all(root.join("contests/tessoku-book/a01")).unwrap();
        fs::write(
            root.join("contests/tessoku-book/metadata.json"),
            r#"{
  "contest": "tessoku-book",
  "tasks": [
    {
      "id": "a01",
      "path": "a01",
      "task_screen_name": "tessoku_book_a01",
      "title": "A01 - The First Problem"
    }
  ]
}
"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn resolves_problem_from_metadata_when_only_problem_is_given() {
        let root = temp_project();
        let args = vec!["a01".to_string()];
        let target = resolve_problem_target(&args, &root, "contests", "test").unwrap();
        assert_eq!(target, ("tessoku-book".to_string(), "a01".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_problem_from_contests_path() {
        let root = temp_project();
        let args = vec!["contests/tessoku-book/a01".to_string()];
        let target = resolve_problem_target(&args, &root, "contests", "test").unwrap();
        assert_eq!(target, ("tessoku-book".to_string(), "a01".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn infers_single_contest_from_metadata() {
        let root = temp_project();
        let contest = infer_contest_target(&root, "contests").unwrap();
        assert_eq!(contest, "tessoku-book");
        fs::remove_dir_all(root).unwrap();
    }
}
