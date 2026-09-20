use crate::atcoder::parser::{
    looks_like_login_page, parse_contest_tasks, parse_problem, parse_submissions,
};
use crate::config::Config;
use crate::models::{Problem, SubmissionSummary, TaskSummary};
use anyhow::{bail, Context, Result};
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER, USER_AGENT};
use reqwest::StatusCode;
use std::{
    io::{self, Write},
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use url::Url;

const BASE_URL: &str = "https://atcoder.jp";
const MAX_FETCH_ATTEMPTS: usize = 4;

pub struct AtCoderClient {
    client: Client,
    cookie_jar: Arc<Jar>,
    config: Config,
}

impl AtCoderClient {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let cookie_jar = Arc::new(Jar::default());
        if let Some(cookie) = &config.revel_session {
            set_revel_session_cookie(&cookie_jar, cookie)?;
        }
        let client = Self::client_builder(cookie_jar.clone())
            .build()
            .context("failed to create HTTP client")?;

        Ok(Self {
            client,
            cookie_jar,
            config,
        })
    }

    fn client_builder(cookie_jar: Arc<Jar>) -> ClientBuilder {
        Client::builder()
            .user_agent("acx/0.1.0")
            .cookie_provider(cookie_jar)
    }

    pub fn contest_tasks(&mut self, contest: &str) -> Result<Vec<TaskSummary>> {
        let url = format!("{BASE_URL}/contests/{contest}/tasks");
        let body = self.get_text_with_session_retry(&url)?;
        let tasks = parse_contest_tasks(contest, &body);

        if tasks.is_empty() {
            bail!("failed to find tasks; check the contest ID: {contest}");
        }

        Ok(tasks)
    }

    pub fn problem(&mut self, contest: &str, task: &TaskSummary) -> Result<Problem> {
        let url = format!(
            "{BASE_URL}/contests/{contest}/tasks/{}",
            task.task_screen_name
        );
        let body = self.get_text_with_session_retry(&url)?;
        parse_problem(&task.id, &body)
    }

    pub fn submissions(
        &mut self,
        contest: &str,
        max_pages: usize,
    ) -> Result<Vec<SubmissionSummary>> {
        let mut submissions = Vec::new();
        for page in 1..=max_pages.max(1) {
            let page_submissions = self.submissions_page(contest, page)?;
            if page_submissions.is_empty() {
                break;
            }
            let first_seen_id = submissions
                .first()
                .map(|submission: &SubmissionSummary| &submission.id);
            if first_seen_id.is_some_and(|id| {
                page_submissions
                    .iter()
                    .any(|submission| &submission.id == id)
            }) {
                break;
            }
            submissions.extend(page_submissions);
        }
        Ok(submissions)
    }

    fn submissions_page(&mut self, contest: &str, page: usize) -> Result<Vec<SubmissionSummary>> {
        let url = if page <= 1 {
            format!("{BASE_URL}/contests/{contest}/submissions/me")
        } else {
            format!("{BASE_URL}/contests/{contest}/submissions/me?page={page}")
        };
        let body = self.get_text_with_session_retry(&url)?;
        Ok(parse_submissions(&body))
    }

    fn get_text_with_session_retry(&mut self, url: &str) -> Result<String> {
        let first = self.get_text(url)?;
        if !looks_like_login_page(&first) {
            return Ok(first);
        }

        self.prompt_revel_session()?;
        let second = self.get_text(url)?;
        if looks_like_login_page(&second) {
            bail!("failed to verify login; check REVEL_SESSION and try again");
        }
        Ok(second)
    }

    fn get_text(&self, url: &str) -> Result<String> {
        for attempt in 1..=MAX_FETCH_ATTEMPTS {
            let response = self
                .client
                .get(url)
                .headers(self.headers()?)
                .send()
                .with_context(|| format!("failed to fetch: {url}"))?;

            if response.status() == StatusCode::TOO_MANY_REQUESTS && attempt < MAX_FETCH_ATTEMPTS {
                let wait = retry_after(&response)
                    .unwrap_or_else(|| Duration::from_secs(2 * attempt as u64));
                sleep(wait);
                continue;
            }

            if !response.status().is_success() {
                bail!("failed to fetch: {} ({url})", response.status());
            }

            return response.text().context("failed to read response body");
        }

        unreachable!("fetch loop always returns or fails")
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("acx/0.1.0"));
        Ok(headers)
    }

    fn prompt_revel_session(&mut self) -> Result<()> {
        eprintln!("AtCoder REVEL_SESSION cookie is required. Copy the value from your browser.");
        print!("REVEL_SESSION: ");
        io::stdout().flush().ok();
        let mut session = String::new();
        io::stdin()
            .read_line(&mut session)
            .context("failed to read REVEL_SESSION")?;
        if session.trim().is_empty() {
            bail!("REVEL_SESSION is empty");
        }

        let session = session.trim().to_string();
        set_revel_session_cookie(&self.cookie_jar, &session)?;
        self.config.revel_session = Some(session);
        self.config.save()
    }
}

fn set_revel_session_cookie(cookie_jar: &Jar, session: &str) -> Result<()> {
    let url = Url::parse(BASE_URL).context("failed to parse AtCoder base URL")?;
    cookie_jar.add_cookie_str(
        &format!("REVEL_SESSION={session}; Domain=atcoder.jp; Path=/; Secure"),
        &url,
    );
    Ok(())
}

fn retry_after(response: &reqwest::blocking::Response) -> Option<Duration> {
    response
        .headers()
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}
