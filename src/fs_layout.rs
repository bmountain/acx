use std::path::{Path, PathBuf};

pub fn contest_dir(contests_dir: &str, contest: &str) -> PathBuf {
    Path::new(contests_dir).join(contest)
}

pub fn problem_dir(contests_dir: &str, contest: &str, problem: &str) -> PathBuf {
    contest_dir(contests_dir, contest).join(problem)
}

pub fn test_dir(contests_dir: &str, contest: &str, problem: &str) -> PathBuf {
    problem_dir(contests_dir, contest, problem).join("test")
}
