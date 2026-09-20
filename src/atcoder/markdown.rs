use regex::{Captures, Regex};

pub fn statement_html_to_markdown(html: String) -> String {
    let html = preserve_math(html);
    let markdown = html2md::parse_html(&html);
    let markdown = normalize_closed_headings(markdown);
    trim_pre_block_trailing_blank_lines(normalize_markdown_math(markdown))
}

fn preserve_math(html: String) -> String {
    let (html, pre_blocks) = protect_pre_blocks(html);
    let html = replace_display_math(html);
    let html = replace_inline_math(html);
    restore_pre_blocks(html, pre_blocks)
}

fn protect_pre_blocks(html: String) -> (String, Vec<String>) {
    let pre_re = Regex::new(r"(?s)<pre\b[^>]*>.*?</pre>").unwrap();
    let mut blocks = Vec::new();
    let html = pre_re
        .replace_all(&html, |captures: &Captures<'_>| {
            let index = blocks.len();
            blocks.push(captures[0].to_string());
            format!("@@ATCODER_RUST_PRE_BLOCK_{index}@@")
        })
        .to_string();
    (html, blocks)
}

fn restore_pre_blocks(mut html: String, blocks: Vec<String>) -> String {
    for (index, block) in blocks.into_iter().enumerate() {
        html = html.replace(&format!("@@ATCODER_RUST_PRE_BLOCK_{index}@@"), &block);
    }
    html
}

fn replace_display_math(html: String) -> String {
    let display_re = Regex::new(
        r#"(?s)<div\b[^>]*style="[^"]*text-align\s*:\s*center;?[^"]*"[^>]*>\s*<var>(.*?)</var>\s*</div>"#,
    )
    .unwrap();

    display_re
        .replace_all(&html, |captures: &Captures<'_>| {
            format!("<p>$$\n{}\n$$</p>", decode_html_entities(&captures[1]))
        })
        .to_string()
}

fn replace_inline_math(html: String) -> String {
    let var_re = Regex::new(r"(?s)<var>(.*?)</var>").unwrap();
    var_re
        .replace_all(&html, |captures: &Captures<'_>| {
            format!("${}$", decode_html_entities(&captures[1]))
        })
        .to_string()
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}

fn normalize_closed_headings(markdown: String) -> String {
    let heading_re = Regex::new(r"(?m)^(#{1,6})\s+(.+?)\s+#+\s*$").unwrap();
    heading_re
        .replace_all(&markdown, |captures: &Captures<'_>| {
            format!("{} {}", &captures[1], captures[2].trim_end())
        })
        .to_string()
}

fn normalize_markdown_math(markdown: String) -> String {
    let mut output = String::with_capacity(markdown.len());
    let mut index = 0;

    while index < markdown.len() {
        let rest = &markdown[index..];
        let Some(delimiter) = math_delimiter(rest) else {
            let ch = rest.chars().next().unwrap();
            output.push(ch);
            index += ch.len_utf8();
            continue;
        };

        output.push_str(delimiter);
        index += delimiter.len();

        let Some(end_offset) = markdown[index..].find(delimiter) else {
            output.push_str(&markdown[index..]);
            break;
        };

        let math = &markdown[index..index + end_offset];
        output.push_str(&normalize_math_content(math));
        output.push_str(delimiter);
        index += end_offset + delimiter.len();
    }

    output
}

fn math_delimiter(text: &str) -> Option<&'static str> {
    if text.starts_with("$$") {
        Some("$$")
    } else if text.starts_with('$') {
        Some("$")
    } else {
        None
    }
}

fn normalize_math_content(text: &str) -> String {
    text.replace(r"\\", r"\").replace(r"\_", "_")
}

fn trim_pre_block_trailing_blank_lines(markdown: String) -> String {
    let code_re = Regex::new(r"(?s)```\n(.*?)\n```").unwrap();
    code_re
        .replace_all(&markdown, |captures: &Captures<'_>| {
            let body = captures[1].trim_end_matches('\n');
            format!("```\n{body}\n```")
        })
        .to_string()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_right_side_heading_markers() {
        let markdown = normalize_closed_headings("### 問題文 ###
text
#### 入力例 1 ####
".to_string());
        assert!(markdown.contains("### 問題文
"));
        assert!(markdown.contains("#### 入力例 1
"));
        assert!(!markdown.contains("問題文 ###"));
    }
}
