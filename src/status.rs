use crate::atcoder::AtCoderClient;
use crate::commands::{StatusFilter, StatusMode, StatusOptions};
use crate::models::{SubmissionSummary, TaskSummary};
use anyhow::Result;
use console::{pad_str, style, Alignment};
use std::collections::HashMap;

pub fn print_contest_status(contest: &str, options: StatusOptions) -> Result<()> {
    let mut client = AtCoderClient::new()?;
    let tasks = client.contest_tasks(contest)?;
    let submissions = client.submissions(contest, options.max_pages)?;

    match options.mode {
        StatusMode::Summary => print_summary(contest, &tasks, &submissions, options.only),
        StatusMode::List => print_list(contest, &submissions, options.only, options.tail),
    }
}

fn print_summary(
    contest: &str,
    tasks: &[TaskSummary],
    submissions: &[SubmissionSummary],
    filter: StatusFilter,
) -> Result<()> {
    let latest = latest_by_problem(tasks, submissions);
    let counts = StatusCounts::from_tasks(tasks, &latest);

    println!("{}", style(format!("{contest} submission status")).bold());
    println!();
    println!(
        "{}: {}  {}: {}  {}: {}  {}: {}  {}: {}",
        style("AC").green(),
        counts.ac,
        style("WA/RE/TLE").red(),
        counts.failed,
        style("CE").yellow(),
        counts.ce,
        style("Pending").cyan(),
        counts.pending,
        style("Unsubmitted").dim(),
        counts.unsubmitted
    );
    println!();
    println!(
        "{} {} {} {}",
        column("task", 5),
        column("submit", 8),
        column("submitted", 20),
        "title"
    );

    for task in tasks {
        let row = StatusRow::from_task(task, latest.get(&task.id));
        if !matches_filter(&row, filter) {
            continue;
        }
        println!(
            "{} {} {} {}",
            column(&row.task_label, 5),
            color_status_column(&row.status, 8),
            column(&format_submitted_at(row.submitted_at.as_deref()), 20),
            row.title
        );
    }

    Ok(())
}

fn print_list(
    contest: &str,
    submissions: &[SubmissionSummary],
    filter: StatusFilter,
    tail: usize,
) -> Result<()> {
    println!("{}", style(format!("{contest} recent submissions")).bold());
    println!();
    if submissions.is_empty() {
        println!("no submissions found");
        return Ok(());
    }

    println!(
        "{} {} {} {}",
        column("submitted", 20),
        column("problem", 24),
        column("status", 8),
        "language"
    );
    for submission in submissions
        .iter()
        .filter(|submission| matches_submission_filter(submission, filter))
        .take(tail)
    {
        println!(
            "{} {} {} {}",
            column(&format_submitted_at(Some(&submission.submitted_at)), 20),
            column(&truncate(&submission.problem, 24), 24),
            color_status_column(&submission.status, 8),
            submission.language
        );
    }

    Ok(())
}

fn latest_by_problem<'a>(
    tasks: &[TaskSummary],
    submissions: &'a [SubmissionSummary],
) -> HashMap<String, &'a SubmissionSummary> {
    let mut aliases = HashMap::new();
    for task in tasks {
        aliases.insert(normalize_problem_key(&task.id), task.id.clone());
        aliases.insert(normalize_problem_key(&task.title), task.id.clone());
        aliases.insert(
            normalize_problem_key(&task.task_screen_name),
            task.id.clone(),
        );
        if let Some(label) = leading_label(&task.title) {
            aliases.insert(normalize_problem_key(label), task.id.clone());
        }
    }

    let mut latest = HashMap::new();
    for submission in submissions {
        let Some(task_id) = task_id_for_submission(&aliases, submission) else {
            continue;
        };
        latest.entry(task_id).or_insert(submission);
    }
    latest
}

fn task_id_for_submission(
    aliases: &HashMap<String, String>,
    submission: &SubmissionSummary,
) -> Option<String> {
    submission
        .task_screen_name
        .as_deref()
        .and_then(|name| aliases.get(&normalize_problem_key(name)))
        .or_else(|| aliases.get(&normalize_problem_key(&submission.problem)))
        .or_else(|| {
            leading_label(&submission.problem)
                .and_then(|label| aliases.get(&normalize_problem_key(label)))
        })
        .cloned()
}

#[derive(Debug)]
struct StatusRow {
    task_label: String,
    title: String,
    status: String,
    submitted_at: Option<String>,
}

impl StatusRow {
    fn from_task(task: &TaskSummary, submission: Option<&&SubmissionSummary>) -> Self {
        let status = submission
            .map(|submission| submission.status.clone())
            .unwrap_or_else(|| "--".to_string());
        let submitted_at = submission.map(|submission| submission.submitted_at.clone());
        Self {
            task_label: leading_label(&task.title)
                .unwrap_or(&task.id)
                .to_ascii_uppercase(),
            title: task.title.clone(),
            status,
            submitted_at,
        }
    }
}

#[derive(Default)]
struct StatusCounts {
    ac: usize,
    failed: usize,
    ce: usize,
    pending: usize,
    unsubmitted: usize,
}

impl StatusCounts {
    fn from_tasks(
        tasks: &[TaskSummary],
        latest: &HashMap<String, &SubmissionSummary>,
    ) -> StatusCounts {
        let mut counts = StatusCounts::default();
        for task in tasks {
            match latest
                .get(&task.id)
                .map(|submission| submission.status.as_str())
            {
                Some("AC") => counts.ac += 1,
                Some("CE") => counts.ce += 1,
                Some(status) if is_pending(status) => counts.pending += 1,
                Some(_) => counts.failed += 1,
                None => counts.unsubmitted += 1,
            }
        }
        counts
    }
}

fn matches_filter(row: &StatusRow, filter: StatusFilter) -> bool {
    match filter {
        StatusFilter::All => true,
        StatusFilter::Failed => is_failed(&row.status),
        StatusFilter::Unsolved => row.status != "AC",
    }
}

fn matches_submission_filter(submission: &SubmissionSummary, filter: StatusFilter) -> bool {
    match filter {
        StatusFilter::All => true,
        StatusFilter::Failed => is_failed(&submission.status),
        StatusFilter::Unsolved => submission.status != "AC",
    }
}

fn color_status(status: &str) -> String {
    if status == "AC" {
        style(status).green().to_string()
    } else if status == "CE" {
        style(status).yellow().to_string()
    } else if status == "--" {
        style(status).dim().to_string()
    } else if is_pending(status) {
        style(status).cyan().to_string()
    } else {
        style(status).red().to_string()
    }
}

fn color_status_column(status: &str, width: usize) -> String {
    let colored = color_status(status);
    format!(
        "{}{}",
        colored,
        " ".repeat(width.saturating_sub(status.len()))
    )
}

fn column(text: &str, width: usize) -> String {
    pad_str(text, width, Alignment::Left, None).to_string()
}

fn format_submitted_at(submitted_at: Option<&str>) -> String {
    let Some(submitted_at) = submitted_at else {
        return "-".to_string();
    };
    submitted_at
        .strip_suffix("+0900")
        .unwrap_or(submitted_at)
        .trim_end()
        .to_string()
}

fn is_failed(status: &str) -> bool {
    status != "AC" && status != "--" && status != "CE" && !is_pending(status)
}

fn is_pending(status: &str) -> bool {
    matches!(
        status,
        "WJ" | "WR" | "Judging" | "Waiting" | "Pending" | "ジャッジ中"
    )
}

fn leading_label(text: &str) -> Option<&str> {
    let label = text.split_whitespace().next()?;
    let label = label.trim_end_matches('-');
    if label.is_empty() {
        None
    } else {
        Some(label)
    }
}

fn normalize_problem_key(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn truncate(text: &str, width: usize) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        if result.chars().count() + 1 >= width {
            result.push('…');
            return result;
        }
        result.push(ch);
    }
    result
}
