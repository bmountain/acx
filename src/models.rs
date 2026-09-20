use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TaskSummary {
    pub id: String,
    pub task_screen_name: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemMetadata {
    pub contest: String,
    pub id: String,
    pub task_screen_name: String,
    pub title: String,
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
