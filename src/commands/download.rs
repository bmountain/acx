use crate::commands::template::write_template;
use crate::models::{ContestMetadata, ContestTaskMetadata, Problem, ProblemMetadata, TaskSummary};
use crate::project_config::ProjectConfig;
use crate::template_gen;
use crate::{atcoder::AtCoderClient, fs_layout, ui};
use anyhow::{bail, Context, Result};
use std::{fs, path::Path, thread};

pub fn download(contest: &str, jobs: usize) -> Result<()> {
    let (project_config, project_root) = ProjectConfig::load_from_current_dir()?;
    let mut client = AtCoderClient::new()?;
    let tasks = client.contest_tasks(contest)?;
    save_contest_metadata(&project_root, &project_config.contests_dir, contest, &tasks)?;
    let total = tasks.len();
    let jobs = jobs.max(1);
    let mut completed = 0usize;
    let mut failures = Vec::new();
    let progress = ui::DownloadProgress::new(total)?;
    progress.status(
        "Downloading",
        format!("{contest} {total} tasks with {jobs} jobs"),
    );
    for chunk in tasks.chunks(jobs) {
        let current_message = chunk
            .first()
            .map(|task| {
                format!(
                    "{} to {}",
                    task.title,
                    fs_layout::problem_dir(&project_config.contests_dir, contest, &task.id)
                        .display()
                )
            })
            .unwrap_or_else(|| contest.to_string());
        progress.status("Fetching", current_message);
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
                    let problem_path =
                        fs_layout::problem_dir(&project_config.contests_dir, contest, &problem.id);
                    progress.status(
                        "Saving",
                        format!("{} to {}", task.title, problem_path.display()),
                    );
                    if let Err(error) = save_problem(
                        &project_root,
                        contest,
                        &task,
                        &problem,
                        &project_config,
                        &progress,
                    ) {
                        let failure = format!("{}: {error:#}", problem.id);
                        progress.warning(format!("failed to download {failure}"));
                        failures.push(failure);
                    }
                    progress.inc();
                }
                Err(error) => {
                    let failure = format!("{error:#}");
                    progress.warning(format!("failed to download {failure}"));
                    failures.push(failure);
                    progress.inc();
                }
            }
        }
    }
    progress.finish(format!("downloaded {completed}/{total} tasks"));

    if !failures.is_empty() {
        ui::error(format!(
            "download finished with {} failures",
            failures.len()
        ));
        bail!("download finished with {} failures", failures.len());
    }
    Ok(())
}

fn save_contest_metadata(
    project_root: &Path,
    contests_dir: &str,
    contest: &str,
    tasks: &[TaskSummary],
) -> Result<()> {
    let contest_dir = project_root.join(fs_layout::contest_dir(contests_dir, contest));
    fs::create_dir_all(&contest_dir)
        .with_context(|| format!("failed to create directory: {}", contest_dir.display()))?;
    let metadata = ContestMetadata {
        contest: contest.to_string(),
        tasks: tasks
            .iter()
            .map(|task| ContestTaskMetadata {
                id: task.id.clone(),
                path: task.id.clone(),
                task_screen_name: task.task_screen_name.clone(),
                title: task.title.clone(),
            })
            .collect(),
    };
    let metadata_json =
        serde_json::to_string_pretty(&metadata).context("failed to serialize contest metadata")?;
    fs::write(
        contest_dir.join("metadata.json"),
        format!("{metadata_json}\n"),
    )
    .with_context(|| {
        format!(
            "failed to write contest metadata: {}",
            contest_dir.display()
        )
    })?;
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
    let problem_dir = project_root.join(fs_layout::problem_dir(
        &config.contests_dir,
        contest,
        &problem.id,
    ));
    let test_dir = project_root.join(fs_layout::test_dir(
        &config.contests_dir,
        contest,
        &problem.id,
    ));
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
        path: problem.id.clone(),
        task_screen_name: task.task_screen_name.clone(),
        title: task.title.clone(),
        statement: "statement.md".to_string(),
        source: config
            .profiles
            .get(&config.default_profile)
            .map(|profile| profile.source.clone())
            .unwrap_or_else(|| "main.cpp".to_string()),
        test_dir: "test".to_string(),
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
        let constraints =
            template_gen::extract_constraints_from_markdown(&problem.statement_markdown);
        let report = write_template(
            &problem_dir,
            &config.template.output,
            problem.input_format.as_deref(),
            constraints.as_deref(),
            config.template.overwrite,
            &problem.id,
        )?;
        if let Some(warning) = &report.warning {
            progress.warning(warning);
        }
        if report.written {
            progress.success("Generated", report.output_path.display().to_string());
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
