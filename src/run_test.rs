use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Debug, Clone)]
pub struct TestReport {
    pub passed: usize,
    pub total: usize,
    pub results: Vec<TestCaseResult>,
    pub first_failure: Option<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestCaseResult {
    pub number: String,
    pub status: TestCaseStatus,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCaseStatus {
    Accepted,
    WrongAnswer,
    RuntimeError,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub input_path: PathBuf,
}

pub fn run_test_cases(test_dir: &Path, command: &str, case_filter: &[usize]) -> Result<TestReport> {
    if !test_dir.exists() {
        bail!("test directory does not exist: {}", test_dir.display());
    }

    let mut inputs = fs::read_dir(test_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("in_") && name.ends_with(".txt"))
        })
        .collect::<Vec<_>>();
    inputs.sort();
    if !case_filter.is_empty() {
        inputs.retain(|path| {
            case_number(path)
                .and_then(|number| number.parse::<usize>().ok())
                .is_some_and(|number| case_filter.contains(&number))
        });
    }

    if inputs.is_empty() {
        bail!("no test input files found: {}", test_dir.display());
    }

    let mut passed = 0usize;
    let mut results = Vec::new();
    let mut first_failure = None;
    for input_path in &inputs {
        let number = case_number(input_path).unwrap_or_default();
        let output_path = test_dir.join(format!("out_{number}.txt"));
        let input = fs::read_to_string(input_path)?;
        let expected = fs::read_to_string(&output_path).with_context(|| {
            format!("failed to read expected output: {}", output_path.display())
        })?;

        let outcome = run_command(command, &input)?;
        if !outcome.success {
            let stderr = non_empty_trimmed(outcome.stderr);
            results.push(TestCaseResult {
                number: number.to_string(),
                status: TestCaseStatus::RuntimeError,
                expected: None,
                actual: None,
                stderr,
            });
            first_failure.get_or_insert_with(|| TestFailure {
                input_path: input_path.to_path_buf(),
            });
            continue;
        }

        if normalize(&outcome.stdout) == normalize(&expected) {
            passed += 1;
            results.push(TestCaseResult {
                number: number.to_string(),
                status: TestCaseStatus::Accepted,
                expected: None,
                actual: None,
                stderr: None,
            });
        } else {
            results.push(TestCaseResult {
                number: number.to_string(),
                status: TestCaseStatus::WrongAnswer,
                expected: Some(expected.trim_end().to_string()),
                actual: Some(outcome.stdout.trim_end().to_string()),
                stderr: non_empty_trimmed(outcome.stderr),
            });
            first_failure.get_or_insert_with(|| TestFailure {
                input_path: input_path.to_path_buf(),
            });
        }
    }

    Ok(TestReport {
        passed,
        total: inputs.len(),
        results,
        first_failure,
    })
}

struct RunOutcome {
    success: bool,
    stdout: String,
    stderr: String,
}

fn run_command(command: &str, input: &str) -> Result<RunOutcome> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start command: {command}"))?;

    {
        let stdin = child.stdin.as_mut().context("failed to open stdin")?;
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    Ok(RunOutcome {
        success: output.status.success(),
        stdout: String::from_utf8(output.stdout).context("stdout is not valid UTF-8")?,
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn non_empty_trimmed(text: String) -> Option<String> {
    let trimmed = text.trim_end();
    if trimmed.trim().is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn case_number(path: &std::path::Path) -> Option<&str> {
    let stem = path.file_stem().and_then(|s| s.to_str())?;
    Some(stem.trim_start_matches("in_"))
}
