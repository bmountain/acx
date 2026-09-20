use crate::atcoder::markdown::statement_html_to_markdown;
use crate::models::{Problem, Sample, SubmissionSummary, TaskSummary};
use anyhow::{anyhow, Result};
use scraper::{ElementRef, Html, Selector};

pub fn parse_contest_tasks(contest: &str, html: &str) -> Vec<TaskSummary> {
    let document = Html::parse_document(html);
    let row_selector = Selector::parse("tbody tr").unwrap();
    let link_selector = Selector::parse("a[href*='/tasks/']").unwrap();

    let mut tasks = Vec::new();
    for row in document.select(&row_selector) {
        let task_links = row
            .select(&link_selector)
            .filter(|link| {
                link.value()
                    .attr("href")
                    .is_some_and(|href| !href.contains("/editorial"))
            })
            .collect::<Vec<_>>();
        let Some(link) = task_links.first() else {
            continue;
        };
        let href = link.value().attr("href").unwrap_or_default();
        let Some(task_screen_name) = href.rsplit('/').next() else {
            continue;
        };
        let title = task_links
            .iter()
            .map(|link| normalize_text(link.text().collect::<Vec<_>>().join(" ")))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" - ");
        let id = problem_id_from_title(&title)
            .unwrap_or_else(|| problem_id_from_task_screen_name(contest, task_screen_name));
        tasks.push(TaskSummary {
            id,
            task_screen_name: task_screen_name.to_string(),
            title,
        });
    }

    tasks
}

fn normalize_text(text: String) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn parse_problem(problem_id: &str, html: &str) -> Result<Problem> {
    let document = Html::parse_document(html);
    let statement_selector = Selector::parse("#task-statement").unwrap();
    let lang_ja_selector = Selector::parse(".lang-ja").unwrap();

    let statement = document
        .select(&statement_selector)
        .next()
        .ok_or_else(|| anyhow!("failed to find statement in HTML"))?;
    let content = statement
        .select(&lang_ja_selector)
        .next()
        .unwrap_or(statement);

    Ok(Problem {
        id: problem_id.to_string(),
        statement_markdown: statement_html_to_markdown(content.html()),
        input_format: extract_input_format(content),
        samples: extract_samples(content),
    })
}

pub fn looks_like_login_page(html: &str) -> bool {
    html.contains("/login") && html.contains("REVEL_SESSION")
}

pub fn parse_submissions(html: &str) -> Vec<SubmissionSummary> {
    let document = Html::parse_document(html);
    let row_selector = Selector::parse("table tbody tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();
    let link_selector = Selector::parse("a").unwrap();
    let status_selector = Selector::parse("span.label").unwrap();

    let mut submissions = Vec::new();
    for row in document.select(&row_selector) {
        let cells = row.select(&cell_selector).collect::<Vec<_>>();
        if cells.len() < 7 {
            continue;
        }

        let submitted_at = normalize_text(cells[0].text().collect::<Vec<_>>().join(" "));
        let problem = normalize_text(cells[1].text().collect::<Vec<_>>().join(" "));
        let task_screen_name = cells[1]
            .select(&link_selector)
            .filter_map(|link| link.value().attr("href"))
            .filter(|href| href.contains("/tasks/"))
            .filter_map(|href| href.rsplit('/').next())
            .next()
            .map(ToString::to_string);
        let language = normalize_text(cells[3].text().collect::<Vec<_>>().join(" "));
        let status = cells[6]
            .select(&status_selector)
            .next()
            .map(|node| normalize_text(node.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_else(|| normalize_text(cells[6].text().collect::<Vec<_>>().join(" ")));
        let id = row
            .select(&link_selector)
            .filter_map(|link| link.value().attr("href"))
            .filter(|href| href.contains("/submissions/"))
            .filter_map(|href| href.rsplit('/').next())
            .find(|part| part.chars().all(|ch| ch.is_ascii_digit()))
            .unwrap_or("")
            .to_string();

        if !id.is_empty() {
            submissions.push(SubmissionSummary {
                id,
                problem,
                task_screen_name,
                language,
                status,
                submitted_at,
            });
        }
    }
    submissions
}

fn extract_samples(statement: ElementRef<'_>) -> Vec<Sample> {
    let h3_selector = Selector::parse("h3").unwrap();
    let pre_selector = Selector::parse("pre").unwrap();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for heading in statement.select(&h3_selector) {
        let label = heading
            .text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();
        if is_sample_input_heading(&label) {
            if let Some(text) = next_pre_text(&heading, &pre_selector) {
                inputs.push(text);
            }
        } else if is_sample_output_heading(&label) {
            if let Some(text) = next_pre_text(&heading, &pre_selector) {
                outputs.push(text);
            }
        }
    }

    inputs
        .into_iter()
        .zip(outputs)
        .map(|(input, output)| Sample { input, output })
        .collect()
}

fn extract_input_format(statement: ElementRef<'_>) -> Option<String> {
    let h3_selector = Selector::parse("h3").unwrap();
    let pre_selector = Selector::parse("pre").unwrap();

    for heading in statement.select(&h3_selector) {
        let label = heading
            .text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();
        if (label.starts_with("入力") || label.starts_with("Input"))
            && !is_sample_input_heading(&label)
        {
            return next_pre_text(&heading, &pre_selector);
        }
    }

    None
}

fn next_pre_text(heading: &ElementRef<'_>, pre_selector: &Selector) -> Option<String> {
    for sibling in heading.next_siblings() {
        let Some(element) = ElementRef::wrap(sibling) else {
            continue;
        };
        if element.value().name() == "h3" {
            return None;
        }
        if element.value().name() == "pre" {
            return Some(element_text(element));
        }
        if let Some(pre) = element.select(pre_selector).next() {
            return Some(element_text(pre));
        }
    }
    None
}

fn element_text(element: ElementRef<'_>) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim_end()
        .to_string()
}

fn is_sample_input_heading(label: &str) -> bool {
    label.starts_with("入力例") || label.starts_with("Sample Input")
}

fn is_sample_output_heading(label: &str) -> bool {
    label.starts_with("出力例") || label.starts_with("Sample Output")
}

fn problem_id_from_task_screen_name(contest: &str, task_screen_name: &str) -> String {
    let normalized_contest = contest.replace('-', "_");
    let prefixes = [format!("{contest}_"), format!("{normalized_contest}_")];

    prefixes
        .iter()
        .find_map(|prefix| task_screen_name.strip_prefix(prefix))
        .unwrap_or(task_screen_name)
        .to_ascii_lowercase()
}

fn problem_id_from_title(title: &str) -> Option<String> {
    let label = title.split_whitespace().next()?;
    let has_digit = label.chars().any(|ch| ch.is_ascii_digit());
    let is_short_ascii = label.len() <= 4
        && label
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-');

    if has_digit && is_short_ascii {
        Some(label.trim_end_matches('-').to_ascii_lowercase())
    } else {
        None
    }
}
