use crate::commands::target::resolve_problem_target;
use crate::models::ProblemMetadata;
use crate::project_config::ProjectConfig;
use crate::{fs_layout, template_gen, ui};
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub fn template(args: &[String], force: bool) -> Result<()> {
    let (config, project_root) = ProjectConfig::load_from_current_dir()?;
    let (contest, problem) =
        resolve_problem_target(args, &project_root, &config.contests_dir, "template")?;
    let problem_dir = project_root.join(fs_layout::problem_dir(
        &config.contests_dir,
        &contest,
        &problem,
    ));
    let metadata = read_problem_metadata(&problem_dir)?;
    let statement_path = problem_dir.join(
        metadata
            .as_ref()
            .map_or("statement.md", |m| m.statement.as_str()),
    );
    let statement = fs::read_to_string(&statement_path)
        .with_context(|| format!("failed to read statement: {}", statement_path.display()))?;
    let input_format = template_gen::extract_input_format_from_markdown(&statement);

    let report = write_template(
        &problem_dir,
        &config.template.output,
        input_format.as_deref(),
        force || config.template.overwrite,
        &problem,
    )?;
    report.print();
    Ok(())
}

pub struct TemplateWriteReport {
    pub output_path: std::path::PathBuf,
    pub warning: Option<String>,
    pub written: bool,
}

impl TemplateWriteReport {
    pub fn print(&self) {
        if let Some(warning) = &self.warning {
            ui::warn(warning);
        }
        if self.written {
            ui::info(format!("Generated {}", self.output_path.display()));
        }
    }
}

pub fn write_template(
    problem_dir: &Path,
    output_name: &str,
    input_format: Option<&str>,
    overwrite: bool,
    problem_id: &str,
) -> Result<TemplateWriteReport> {
    let output_path = problem_dir.join(output_name);
    if output_path.exists() && !overwrite {
        return Ok(TemplateWriteReport {
            output_path,
            warning: None,
            written: false,
        });
    }

    let generated = template_gen::generate_cpp(input_format);
    let warning = generated
        .warning
        .as_ref()
        .map(|warning| format!("failed to parse input format for {problem_id}: {warning}"));
    fs::write(&output_path, generated.code)
        .with_context(|| format!("failed to write template: {}", output_path.display()))?;
    Ok(TemplateWriteReport {
        output_path,
        warning,
        written: true,
    })
}

fn read_problem_metadata(problem_dir: &Path) -> Result<Option<ProblemMetadata>> {
    let path = problem_dir.join("metadata.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read metadata: {}", path.display()))?;
    let metadata = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse metadata: {}", path.display()))?;
    Ok(Some(metadata))
}
