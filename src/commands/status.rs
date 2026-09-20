use crate::commands::target::infer_contest_target;
use crate::project_config::ProjectConfig;
use crate::status as status_workflow;
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum StatusMode {
    Summary,
    List,
}

#[derive(Debug, Clone, Copy)]
pub enum StatusFilter {
    All,
    Failed,
    Unsolved,
}

#[derive(Debug, Clone, Copy)]
pub struct StatusOptions {
    pub mode: StatusMode,
    pub only: StatusFilter,
    pub tail: usize,
    pub max_pages: usize,
}

pub fn status(contest: Option<&str>, options: StatusOptions) -> Result<()> {
    let (_, project_root) = ProjectConfig::load_from_current_dir()?;
    let contest = match contest {
        Some(contest) => contest.to_string(),
        None => infer_contest_target(&project_root)?,
    };
    status_workflow::print_contest_status(&contest, options)
}
