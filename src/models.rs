use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TaskSummary {
    pub id: String,
    pub task_screen_name: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestMetadata {
    pub contest: String,
    #[serde(default)]
    pub tasks: Vec<ContestTaskMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestTaskMetadata {
    pub id: String,
    pub path: String,
    pub task_screen_name: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemMetadata {
    pub contest: String,
    pub id: String,
    #[serde(default)]
    pub path: String,
    pub task_screen_name: String,
    pub title: String,
    #[serde(default = "default_statement_path")]
    pub statement: String,
    #[serde(default = "default_source_path")]
    pub source: String,
    #[serde(default = "default_test_dir")]
    pub test_dir: String,
}

fn default_statement_path() -> String {
    "statement.md".to_string()
}

fn default_source_path() -> String {
    "main.cpp".to_string()
}

fn default_test_dir() -> String {
    "test".to_string()
}

#[derive(Debug, Clone)]
pub struct Problem {
    pub id: String,
    pub statement_markdown: String,
    pub input_format: Option<String>,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct SubmissionSummary {
    pub id: String,
    pub problem: String,
    pub task_screen_name: Option<String>,
    pub language: String,
    pub status: String,
    pub submitted_at: String,
}
