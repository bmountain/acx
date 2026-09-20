use std::path::{Path, PathBuf};

pub fn contest_dir(contest: &str) -> PathBuf {
    Path::new("contests").join(contest)
}

pub fn problem_dir(contest: &str, problem: &str) -> PathBuf {
    contest_dir(contest).join(problem)
}

pub fn test_dir(contest: &str, problem: &str) -> PathBuf {
    problem_dir(contest, problem).join("test")
}
