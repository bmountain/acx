use anyhow::{anyhow, Result};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct GenerateResult {
    pub code: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
enum Type {
    Int,
    LongLong,
    String,
    Char,
}

#[derive(Debug, Clone)]
enum Item {
    Scalar {
        name: String,
        ty: Type,
    },
    Vector {
        name: String,
        len: String,
        ty: Type,
    },
    RepeatedRows {
        len: String,
        columns: Vec<(String, Type)>,
    },
    Matrix {
        name: String,
        rows: String,
        cols: String,
        ty: Type,
    },
}

pub fn generate_cpp(input_format: Option<&str>) -> GenerateResult {
    generate_cpp_with_constraints(input_format, None)
}

pub fn generate_cpp_with_constraints(
    input_format: Option<&str>,
    constraints: Option<&str>,
) -> GenerateResult {
    let type_context = TypeContext::from_constraints(constraints.unwrap_or(""));
    match input_format
        .ok_or_else(|| anyhow!("input format was not found"))
        .and_then(|input| parse_input_spec(input, &type_context))
    {
        Ok(items) => GenerateResult {
            code: render_cpp(&items),
            warning: None,
        },
        Err(error) => GenerateResult {
            code: default_cpp(),
            warning: Some(format!("{error}; generated default main.cpp")),
        },
    }
}

#[derive(Debug, Default)]
struct TypeContext {
    long_long_vars: Vec<String>,
}

impl TypeContext {
    fn from_constraints(constraints: &str) -> Self {
        let mut long_long_vars = Vec::new();
        for line in constraints.lines() {
            if !line_requires_long_long(line) {
                continue;
            }
            for name in variable_names(line) {
                if !long_long_vars.contains(&name) {
                    long_long_vars.push(name);
                }
            }
        }
        Self { long_long_vars }
    }

    fn scalar_type(&self, name: &str) -> Type {
        if self.long_long_vars.iter().any(|var| var == name) {
            Type::LongLong
        } else {
            scalar_type(name)
        }
    }
}

fn line_requires_long_long(line: &str) -> bool {
    let line = line.replace(' ', "");
    Regex::new(r"10\^\{?(1[0-9]|[2-9][0-9])\}?")
        .unwrap()
        .is_match(&line)
        || Regex::new(r"[1-9][0-9]{9,}").unwrap().is_match(&line)
}

fn variable_names(line: &str) -> Vec<String> {
    Regex::new(r"[A-Za-z][A-Za-z0-9]*(?:_[A-Za-z0-9]+|_\{[^}]+\})?")
        .unwrap()
        .find_iter(line)
        .filter_map(|mat| {
            let token = mat.as_str();
            let name = token.split_once('_').map_or(token, |(name, _)| name);
            if is_ident(name) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn parse_input_spec(input: &str, type_context: &TypeContext) -> Result<Vec<Item>> {
    let lines = input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(anyhow!("input format is empty"));
    }

    let mut items = Vec::new();
    let first_vars = parse_plain_vars(lines[0])?;
    for var in &first_vars {
        items.push(Item::Scalar {
            name: var.clone(),
            ty: type_context.scalar_type(var),
        });
    }

    let mut index = 1usize;
    while index < lines.len() {
        let line = lines[index];
        if index + 2 < lines.len() && is_dots_line(lines[index + 1]) {
            if let Some((name, rows, cols)) = parse_matrix_rows(line, lines[index + 2]) {
                let ty = matrix_type(&name);
                items.push(Item::Matrix {
                    name,
                    rows,
                    cols,
                    ty,
                });
            } else {
                let Some((len, columns)) = parse_repeated_rows(line, lines[index + 2]) else {
                    return Err(anyhow!("unsupported repeated input format"));
                };
                items.push(Item::RepeatedRows { len, columns });
            }
            index += 3;
        } else if is_vector_line(line) {
            if let Ok((name, rows, cols)) = parse_matrix_single_line(line) {
                let ty = matrix_type(&name);
                items.push(Item::Matrix {
                    name,
                    rows,
                    cols,
                    ty,
                });
            } else {
                let (name, len) = parse_vector_line(line)?;
                let ty = vector_type(&name);
                items.push(Item::Vector { name, len, ty });
            }
            index += 1;
        } else {
            for var in parse_plain_vars(line)? {
                items.push(Item::Scalar {
                    name: var.clone(),
                    ty: type_context.scalar_type(&var),
                });
            }
            index += 1;
        }
    }

    Ok(items)
}

fn parse_plain_vars(line: &str) -> Result<Vec<String>> {
    let vars = line
        .split_whitespace()
        .filter(|token| is_ident(token))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if vars.is_empty() || vars.len() != line.split_whitespace().count() {
        Err(anyhow!("unsupported scalar line: {line}"))
    } else {
        Ok(vars)
    }
}

fn is_vector_line(line: &str) -> bool {
    line.contains("...") || line.contains('…') || line.contains("\\cdots")
}

fn parse_vector_line(line: &str) -> Result<(String, String)> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 3
        || !tokens
            .iter()
            .any(|token| matches!(*token, "..." | "…" | "\\cdots"))
    {
        return Err(anyhow!("unsupported vector line: {line}"));
    }
    let first = parse_subscripted(tokens[0])?;
    let last = parse_subscripted(tokens[tokens.len() - 1])?;
    if first.0 != last.0 {
        return Err(anyhow!("vector endpoints use different names: {line}"));
    }
    Ok((first.0, last.1))
}

fn parse_matrix_single_line(line: &str) -> Result<(String, String, String)> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 3
        || !tokens
            .iter()
            .any(|token| matches!(*token, "..." | "…" | "\\cdots"))
    {
        return Err(anyhow!("unsupported matrix line: {line}"));
    }
    let first = parse_matrix_subscripted(tokens[0])?;
    let last = parse_matrix_subscripted(tokens[tokens.len() - 1])?;
    if first.name != last.name {
        return Err(anyhow!("matrix endpoints use different names: {line}"));
    }
    Ok((first.name, last.row, last.col))
}

fn parse_matrix_rows(start: &str, end: &str) -> Option<(String, String, String)> {
    let start_tokens = start.split_whitespace().collect::<Vec<_>>();
    let end_tokens = end.split_whitespace().collect::<Vec<_>>();
    let start_first = parse_matrix_subscripted(start_tokens.first()?).ok()?;
    let start_last = parse_matrix_subscripted(start_tokens.last()?).ok()?;
    let end_first = parse_matrix_subscripted(end_tokens.first()?).ok()?;
    let end_last = parse_matrix_subscripted(end_tokens.last()?).ok()?;

    if start_first.name != start_last.name
        || start_first.name != end_first.name
        || start_first.name != end_last.name
    {
        return None;
    }
    if start_first.row != start_last.row || end_first.row != end_last.row {
        return None;
    }
    if start_first.col != end_first.col || start_last.col != end_last.col {
        return None;
    }
    Some((start_first.name, end_first.row, start_last.col))
}

#[derive(Debug)]
struct MatrixSubscript {
    name: String,
    row: String,
    col: String,
}

fn parse_matrix_subscripted(token: &str) -> Result<MatrixSubscript> {
    let cleaned = token.trim_matches(|ch: char| ch == ',' || ch == '$');
    let Some((name, index)) = cleaned.split_once('_') else {
        return Err(anyhow!("expected subscripted variable: {token}"));
    };
    if !is_ident(name) {
        return Err(anyhow!("invalid variable name: {name}"));
    }
    let index = index.trim_start_matches('{').trim_end_matches('}');
    let Some((row, col)) = index.split_once(',') else {
        return Err(anyhow!("expected matrix subscript: {token}"));
    };
    if row.is_empty() || col.is_empty() {
        return Err(anyhow!("empty matrix subscript: {token}"));
    }
    Ok(MatrixSubscript {
        name: name.to_string(),
        row: row.to_string(),
        col: col.to_string(),
    })
}

fn parse_repeated_rows(start: &str, end: &str) -> Option<(String, Vec<(String, Type)>)> {
    let start_vars = start
        .split_whitespace()
        .map(parse_subscripted)
        .collect::<Result<Vec<_>>>()
        .ok()?;
    let end_vars = end
        .split_whitespace()
        .map(parse_subscripted)
        .collect::<Result<Vec<_>>>()
        .ok()?;
    if start_vars.len() != end_vars.len() || start_vars.is_empty() {
        return None;
    }
    let len = end_vars[0].1.clone();
    let mut columns = Vec::new();
    for ((start_name, _), (end_name, end_index)) in start_vars.iter().zip(end_vars.iter()) {
        if start_name != end_name || *end_index != len {
            return None;
        }
        columns.push((
            start_name.clone(),
            repeated_type(start_name, start_vars.len()),
        ));
    }
    Some((len, columns))
}

fn vector_type(name: &str) -> Type {
    if name.starts_with('S') {
        Type::String
    } else {
        Type::LongLong
    }
}

fn matrix_type(name: &str) -> Type {
    if name == "C" {
        Type::Char
    } else {
        Type::LongLong
    }
}

fn repeated_type(name: &str, column_count: usize) -> Type {
    if column_count == 1 && name.starts_with('S') {
        Type::String
    } else {
        Type::LongLong
    }
}

fn parse_subscripted(token: &str) -> Result<(String, String)> {
    let cleaned = token.trim_matches(|ch: char| ch == ',' || ch == '$');
    let Some((name, index)) = cleaned.split_once('_') else {
        return Err(anyhow!("expected subscripted variable: {token}"));
    };
    if !is_ident(name) {
        return Err(anyhow!("invalid variable name: {name}"));
    }
    let index = index
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_string();
    if index.is_empty() {
        return Err(anyhow!("empty subscript: {token}"));
    }
    if index.contains(',') {
        return Err(anyhow!(
            "multi-dimensional subscript is unsupported: {token}"
        ));
    }
    Ok((name.to_string(), index))
}

fn is_dots_line(line: &str) -> bool {
    matches!(line.trim(), "..." | "…" | "\\vdots" | ":" | "：")
}

fn is_ident(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric())
}

fn scalar_type(name: &str) -> Type {
    if matches!(name, "N" | "M" | "H" | "W" | "Q" | "K" | "T" | "D") {
        Type::Int
    } else if name.starts_with('S') {
        Type::String
    } else {
        Type::LongLong
    }
}

fn render_cpp(items: &[Item]) -> String {
    let mut main_lines = Vec::new();
    let mut args = Vec::new();
    let mut call_args = Vec::new();

    for item in items {
        match item {
            Item::Scalar { name, ty } => {
                main_lines.push(format!("    {} {};", cpp_type(ty), name));
                main_lines.push(format!("    cin >> {};", name));
                args.push(format!("{} {}", cpp_type(ty), name));
                call_args.push(name.clone());
            }
            Item::Vector { name, len, ty } => {
                main_lines.push(format!(
                    "    vector<{}> {}({} + 1);",
                    cpp_type(ty),
                    name,
                    len
                ));
                main_lines.push(format!("    for (int i = 1; i <= {len}; i++) {{"));
                main_lines.push(format!("        cin >> {name}[i];"));
                main_lines.push("    }".to_string());
                args.push(format!("const vector<{}>& {}", cpp_type(ty), name));
                call_args.push(name.clone());
            }
            Item::RepeatedRows { len, columns } => {
                for (name, ty) in columns {
                    main_lines.push(format!(
                        "    vector<{}> {}({} + 1);",
                        cpp_type(ty),
                        name,
                        len
                    ));
                    args.push(format!("const vector<{}>& {}", cpp_type(ty), name));
                    call_args.push(name.clone());
                }
                main_lines.push(format!("    for (int i = 1; i <= {len}; i++) {{"));
                let row_read = columns
                    .iter()
                    .map(|(name, _)| format!("{name}[i]"))
                    .collect::<Vec<_>>()
                    .join(" >> ");
                main_lines.push(format!("        cin >> {row_read};"));
                main_lines.push("    }".to_string());
            }
            Item::Matrix {
                name,
                rows,
                cols,
                ty,
            } => {
                main_lines.push(format!(
                    "    vector<vector<{}>> {}({} + 1, vector<{}>({} + 1));",
                    cpp_type(ty),
                    name,
                    rows,
                    cpp_type(ty),
                    cols
                ));
                main_lines.push(format!("    for (int i = 1; i <= {rows}; i++) {{"));
                main_lines.push(format!("        for (int j = 1; j <= {cols}; j++) {{"));
                main_lines.push(format!("            cin >> {name}[i][j];"));
                main_lines.push("        }".to_string());
                main_lines.push("    }".to_string());
                args.push(format!("const vector<vector<{}>>& {}", cpp_type(ty), name));
                call_args.push(name.clone());
            }
        }
    }

    let solve_args = args.join(", ");
    let call = call_args.join(", ");
    let main_lines = main_lines.join("\n");

    format!(
        "{}\nusing namespace std;\n\nusing ll = long long;\nusing ull = unsigned long long;\nusing pii = pair<int, int>;\nusing pll = pair<ll, ll>;\n\ntemplate <class T>\nusing vec = vector<T>;\n\nvoid solve({solve_args}) {{\n    // TODO: implement\n}}\n\nint main() {{\n    ios::sync_with_stdio(false);\n    cin.tie(nullptr);\n\n{main_lines}\n\n    solve({call});\n    return 0;\n}}\n",
        common_includes()
    )
}

pub fn extract_constraints_from_markdown(markdown: &str) -> Option<String> {
    let mut in_constraints_section = false;
    let mut lines = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            if in_constraints_section {
                break;
            }
            in_constraints_section = trimmed.contains("制約") || trimmed.contains("Constraints");
            continue;
        }

        if in_constraints_section && !trimmed.is_empty() {
            lines.push(line.to_string());
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

pub fn extract_input_format_from_markdown(markdown: &str) -> Option<String> {
    let mut in_input_section = false;
    let mut in_code_block = false;
    let mut block = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            in_input_section = trimmed.contains("入力") || trimmed.contains("Input");
            in_code_block = false;
            block.clear();
            continue;
        }

        if !in_input_section {
            continue;
        }

        if trimmed.starts_with("```") {
            if in_code_block {
                return Some(block.join("\n"));
            }
            in_code_block = true;
            block.clear();
            continue;
        }

        if in_code_block {
            block.push(line.to_string());
        }
    }

    None
}

fn cpp_type(ty: &Type) -> &'static str {
    match ty {
        Type::Int => "int",
        Type::LongLong => "long long",
        Type::String => "string",
        Type::Char => "char",
    }
}

fn default_cpp() -> String {
    format!(
        "{}\nusing namespace std;\n\nusing ll = long long;\nusing ull = unsigned long long;\nusing pii = pair<int, int>;\nusing pll = pair<ll, ll>;\n\ntemplate <class T>\nusing vec = vector<T>;\n\nvoid solve() {{\n    // TODO: implement\n}}\n\nint main() {{\n    ios::sync_with_stdio(false);\n    cin.tie(nullptr);\n\n    // TODO: Fix input reading. The input format parser could not generate it automatically.\n    solve();\n    return 0;\n}}\n",
        common_includes()
    )
}

fn common_includes() -> &'static str {
    "#include <bits/stdc++.h>"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_vector_input() {
        let generated = generate_cpp(Some("N\nA_1 A_2 \\cdots A_N\n"));
        assert!(generated.warning.is_none());
        assert!(generated.code.contains("int N;"));
        assert!(generated.code.contains("#include <bits/stdc++.h>"));
        assert!(generated.code.contains("vector<long long> A(N + 1);"));
        assert!(generated.code.contains("for (int i = 1; i <= N; i++)"));
        assert!(generated.code.contains("cin >> A[i];"));
        assert!(generated
            .code
            .contains("void solve(int N, const vector<long long>& A)"));
    }

    #[test]
    fn generates_string_vector_input() {
        let generated = generate_cpp(Some(
            "H
S_1 S_2 \\cdots S_H
",
        ));
        assert!(generated.warning.is_none());
        assert!(generated.code.contains("vector<string> S(H + 1);"));
        assert!(generated.code.contains("cin >> S[i];"));
        assert!(generated
            .code
            .contains("void solve(int H, const vector<string>& S)"));
    }

    #[test]
    fn generates_repeated_row_input() {
        let generated = generate_cpp(Some("N\nA_1 B_1\n...\nA_N B_N\n"));
        assert!(generated.warning.is_none());
        assert!(generated.code.contains("vector<long long> A(N + 1);"));
        assert!(generated.code.contains("vector<long long> B(N + 1);"));
        assert!(generated.code.contains("cin >> A[i] >> B[i];"));
    }

    #[test]
    fn generates_multiple_blocks() {
        let generated = generate_cpp(Some("N Q\nA_1 A_2 \\cdots A_N\nL_1 R_1\n:\nL_Q R_Q\n"));
        assert!(generated.warning.is_none());
        assert!(generated.code.contains("int N;"));
        assert!(generated.code.contains("int Q;"));
        assert!(generated.code.contains("vector<long long> A(N + 1);"));
        assert!(generated.code.contains("vector<long long> L(Q + 1);"));
        assert!(generated.code.contains("vector<long long> R(Q + 1);"));
        assert!(generated.code.contains("cin >> L[i] >> R[i];"));
    }

    #[test]
    fn generates_scalar_lines_before_repeated_rows() {
        let generated = generate_cpp(Some("D\nN\nL_1 R_1\n\\vdots\nL_N R_N\n"));
        assert!(generated.warning.is_none());
        assert!(generated.code.contains("int D;"));
        assert!(generated.code.contains("int N;"));
        assert!(generated.code.contains("vector<long long> L(N + 1);"));
        assert!(generated.code.contains("vector<long long> R(N + 1);"));
    }

    #[test]
    fn reads_scalars_before_allocating_vectors() {
        let generated = generate_cpp(Some("N K\nP_1 P_2 \\cdots P_N\nQ_1 Q_2 \\cdots Q_N\n"));
        assert!(generated.warning.is_none());
        let read_n = generated.code.find("cin >> N;").unwrap();
        let vector_p = generated.code.find("vector<long long> P(N + 1);").unwrap();
        assert!(read_n < vector_p);
    }

    #[test]
    fn generates_single_line_matrix_input() {
        let generated = generate_cpp(Some(
            "N
A_{1,1} ... A_{N,N}
",
        ));
        assert!(generated.warning.is_none());
        assert!(generated
            .code
            .contains("vector<vector<long long>> A(N + 1, vector<long long>(N + 1));"));
        assert!(generated.code.contains("for (int i = 1; i <= N; i++)"));
        assert!(generated.code.contains("for (int j = 1; j <= N; j++)"));
        assert!(generated.code.contains("cin >> A[i][j];"));
    }

    #[test]
    fn generates_matrix_rows_input() {
        let generated = generate_cpp(Some(
            "H W
A_{1,1} ... A_{1,W}
...
A_{H,1} ... A_{H,W}
",
        ));
        assert!(generated.warning.is_none());
        assert!(generated
            .code
            .contains("vector<vector<long long>> A(H + 1, vector<long long>(W + 1));"));
    }

    #[test]
    fn generates_char_matrix_for_c_grid() {
        let generated = generate_cpp(Some(
            "H W
C_{1,1} ... C_{1,W}
...
C_{H,1} ... C_{H,W}
",
        ));
        assert!(generated.warning.is_none());
        assert!(generated
            .code
            .contains("vector<vector<char>> C(H + 1, vector<char>(W + 1));"));
    }

    #[test]
    fn infers_long_long_scalar_from_constraints() {
        let generated = generate_cpp_with_constraints(
            Some("N W\n"),
            Some("* $1 \\leq N \\leq 2 \\times 10^5$\n* $1 \\leq W \\leq 10^{18}$"),
        );
        assert!(generated.warning.is_none());
        assert!(generated.code.contains("int N;"));
        assert!(generated.code.contains("long long W;"));
        assert!(generated.code.contains("void solve(int N, long long W)"));
    }

    #[test]
    fn extracts_constraints_from_markdown() {
        let constraints = extract_constraints_from_markdown(
            "### 問題文\ntext\n### 制約\n* $1 \\leq W \\leq 10^{18}$\n### 入力\n```\nW\n```\n",
        )
        .unwrap();
        assert!(constraints.contains("W"));
        assert!(!constraints.contains("```"));
    }

    #[test]
    fn falls_back_for_variable_length_rows() {
        let generated = generate_cpp(Some(
            "N
X_{1,1} ... X_{1,K_1}
...
X_{N,1} ... X_{N,K_N}
",
        ));
        assert!(generated.warning.is_some());
        assert!(generated.code.contains("void solve()"));
        assert!(generated.code.contains("TODO: Fix input reading"));
    }
}
