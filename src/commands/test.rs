use crate::commands::target::resolve_problem_target;
use crate::models::ProblemMetadata;
use crate::project_config::ProjectConfig;
use crate::{builder::BuildPlan, fs_layout, run_test, ui};
use anyhow::{bail, Context, Result};
use console::style;
use std::fs;

pub fn test(args: &[String], profile_name: Option<&str>, case_filter: &[usize]) -> Result<()> {
    let (config, project_root) = ProjectConfig::load_from_current_dir()?;
    let (contest, problem) = resolve_problem_target(args, &project_root, "test")?;
    let (_, profile) = config.profile(profile_name)?;
    let problem_dir = project_root.join(fs_layout::problem_dir(&contest, &problem));
    let build_plan = BuildPlan::new(&problem_dir, profile);

    println!(
        "{} {}",
        style("Building").cyan().bold(),
        build_plan.source.display()
    );
    println!("{}", build_plan.build_command());
    build_plan.build()?;
    println!("{}", style("Running tests").cyan().bold());
    let report = run_test::run_test_cases(
        &problem_dir.join("test"),
        &build_plan.run_command,
        case_filter,
    )?;
    print_report(&report);

    if report.passed == report.total {
        println!();
        ui::success(format!(
            "All tests passed: {}/{}",
            report.passed, report.total
        ));
        println!(
            "{} {}",
            style("Submit URL").cyan().bold(),
            submit_url(&contest, &problem_dir)?
        );
        return Ok(());
    }

    println!();
    print_failure_details(&report);
    if let Some(failure) = &report.first_failure {
        print_debug_command(&build_plan.run_command, &failure.input_path);
    }

    bail!("tests failed: {}/{}", report.passed, report.total)
}

fn print_report(report: &run_test::TestReport) {
    println!();
    println!("{:<6} {:<8}", "case", "result");
    for result in &report.results {
        println!(
            "{:<6} {:<8}",
            format!("#{}", result.number),
            color_test_status(result.status)
        );
    }
}

fn print_failure_details(report: &run_test::TestReport) {
    for result in &report.results {
        match result.status {
            run_test::TestCaseStatus::Accepted => {}
            run_test::TestCaseStatus::WrongAnswer => {
                ui::caution(format!("Case #{}: WA", result.number));
                if let Some(expected) = &result.expected {
                    let actual = result.actual.as_deref().unwrap_or("");
                    print_diff(expected, actual);
                }
                if let Some(stderr) = &result.stderr {
                    println!("{}", style("stderr").dim());
                    println!("{stderr}");
                }
                println!();
            }
            run_test::TestCaseStatus::RuntimeError => {
                ui::caution(format!("Case #{}: RE", result.number));
                if let Some(stderr) = &result.stderr {
                    println!("{}", style("stderr").dim());
                    println!("{stderr}");
                }
                println!();
            }
        }
    }
}

fn color_test_status(status: run_test::TestCaseStatus) -> String {
    match status {
        run_test::TestCaseStatus::Accepted => style("AC").green().bold().to_string(),
        run_test::TestCaseStatus::WrongAnswer => style("WA").red().bold().to_string(),
        run_test::TestCaseStatus::RuntimeError => style("RE").red().bold().to_string(),
    }
}

fn print_diff(expected: &str, actual: &str) {
    let expected_lines = expected.lines().collect::<Vec<_>>();
    let actual_lines = actual.lines().collect::<Vec<_>>();
    let max_len = expected_lines.len().max(actual_lines.len());
    let first_diff = (0..max_len)
        .find(|&index| expected_lines.get(index) != actual_lines.get(index))
        .unwrap_or(0);
    let start = first_diff.saturating_sub(2);
    let end = (first_diff + 3).min(max_len);

    for index in start..end {
        let line_no = index + 1;
        match (expected_lines.get(index), actual_lines.get(index)) {
            (Some(expected), Some(actual)) if expected == actual => {
                println!(" {:>4}  {}", line_no, expected);
            }
            (Some(expected), Some(actual)) => {
                println!("{}{:>4}  {}", style("-").red(), line_no, expected);
                println!("{}{:>4}  {}", style("+").green(), line_no, actual);
            }
            (Some(expected), None) => {
                println!("{}{:>4}  {}", style("-").red(), line_no, expected);
            }
            (None, Some(actual)) => {
                println!("{}{:>4}  {}", style("+").green(), line_no, actual);
            }
            (None, None) => {}
        }
    }
}

fn print_debug_command(run_command: &str, input_path: &std::path::Path) {
    println!("{}", style("To debug with gdb:").cyan().bold());
    println!("gdb --args {run_command}");
    println!("(gdb) run < {}", shell_arg(input_path));
}

fn submit_url(contest: &str, problem_dir: &std::path::Path) -> Result<String> {
    let metadata_path = problem_dir.join("metadata.json");
    if !metadata_path.exists() {
        return Ok(format!("https://atcoder.jp/contests/{contest}/submit"));
    }
    let metadata = fs::read_to_string(&metadata_path)
        .with_context(|| format!("failed to read metadata: {}", metadata_path.display()))?;
    let metadata: ProblemMetadata = serde_json::from_str(&metadata)
        .with_context(|| format!("failed to parse metadata: {}", metadata_path.display()))?;
    Ok(format!(
        "https://atcoder.jp/contests/{contest}/submit?taskScreenName={}",
        metadata.task_screen_name
    ))
}

fn shell_arg(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}
