use crate::commands::template::write_template;
use crate::models::{Problem, ProblemMetadata, TaskSummary};
use crate::project_config::ProjectConfig;
use crate::{atcoder::AtCoderClient, fs_layout, ui};
use anyhow::{bail, Context, Result};
use std::{fs, path::Path, thread};

pub fn download(contest: &str, jobs: usize) -> Result<()> {
    let (project_config, project_root) = ProjectConfig::load_from_current_dir()?;
    let mut client = AtCoderClient::new()?;
    let tasks = client.contest_tasks(contest)?;
    let total = tasks.len();
    let jobs = jobs.max(1);
    let mut completed = 0usize;
    let mut failures = Vec::new();
    let progress = ui::DownloadProgress::new(total)?;
    progress.status(format!("Preparing {total} tasks with {jobs} jobs"));
    for chunk in tasks.chunks(jobs) {
        let current_message = chunk
            .first()
            .map(|task| format!("Downloading \"{}\"", task.title))
            .unwrap_or_else(|| "Downloading".to_string());
        progress.status(current_message.clone());
        let fetched = thread::scope(|scope| {
            chunk
                .iter()
                .cloned()
                .map(|task| {
                    scope.spawn(move || {
                        let mut client = AtCoderClient::new()?;
                        let problem = client
                            .problem(contest, &task)
                            .with_context(|| format!("failed to fetch problem: {}", task.id))?;
                        Ok::<(TaskSummary, Problem), anyhow::Error>((task, problem))
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("download worker panicked"))
                .collect::<Vec<_>>()
        });

        for result in fetched {
            completed += 1;
            match result {
                Ok((task, problem)) => {
                    if let Err(error) = save_problem(
                        &project_root,
                        contest,
                        &task,
                        &problem,
                        &project_config,
                        &progress,
                    ) {
                        failures.push(format!("{}: {error:#}", problem.id));
                    }
                    progress.inc();
                }
                Err(error) => {
                    failures.push(format!("{error:#}"));
                    progress.inc();
                }
            }
        }
    }
    progress.finish(format!("Downloaded {completed} tasks."));

    for failure in &failures {
        ui::warn(format!("failed to download {failure}"));
    }
    if !failures.is_empty() {
        bail!("download finished with {} failures", failures.len());
    }
    Ok(())
}

fn save_problem(
    project_root: &Path,
    contest: &str,
    task: &TaskSummary,
    problem: &crate::models::Problem,
    config: &ProjectConfig,
    progress: &ui::DownloadProgress,
) -> Result<()> {
    let problem_dir = project_root.join(fs_layout::problem_dir(contest, &problem.id));
    let test_dir = project_root.join(fs_layout::test_dir(contest, &problem.id));
    fs::create_dir_all(&test_dir)
        .with_context(|| format!("failed to create directory: {}", test_dir.display()))?;
    remove_old_test_cases(&test_dir)?;

    fs::write(
        problem_dir.join("statement.md"),
        &problem.statement_markdown,
    )
    .with_context(|| format!("failed to write statement: {}", problem_dir.display()))?;
    let metadata = ProblemMetadata {
        contest: contest.to_string(),
        id: problem.id.clone(),
        task_screen_name: task.task_screen_name.clone(),
        title: task.title.clone(),
    };
    let metadata_json =
        serde_json::to_string_pretty(&metadata).context("failed to serialize metadata")?;
    fs::write(
        problem_dir.join("metadata.json"),
        format!("{metadata_json}\n"),
    )
    .with_context(|| format!("failed to write metadata: {}", problem_dir.display()))?;
    for (index, sample) in problem.samples.iter().enumerate() {
        let n = index + 1;
        fs::write(
            test_dir.join(format!("in_{n}.txt")),
            ensure_trailing_newline(&sample.input),
        )?;
        fs::write(
            test_dir.join(format!("out_{n}.txt")),
            ensure_trailing_newline(&sample.output),
        )?;
    }

    if config.template.enabled_on_download {
        let report = write_template(
            &problem_dir,
            &config.template.output,
            problem.input_format.as_deref(),
            config.template.overwrite,
            &problem.id,
        )?;
        if let Some(warning) = &report.warning {
            progress.warning(warning);
        }
        if report.written {
            progress.status(format!("Generated {}", report.output_path.display()));
        }
    }

    Ok(())
}

fn remove_old_test_cases(test_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(test_dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with("in_") || name.starts_with("out_") {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove old test case: {}", path.display()))?;
        }
    }
    Ok(())
}

fn ensure_trailing_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}
