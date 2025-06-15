use crate::services::markdown::render_markdown_to_lines;
use ratatui::style::Color;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use regex::Regex;
use serde_json::Value;
#[cfg(test)]
use stakpak_shared::models::integrations::openai::FunctionCall;
use stakpak_shared::models::integrations::openai::ToolCall;
use uuid::Uuid;
#[derive(Clone)]
pub struct BubbleColors {
    pub border_color: Color,
    pub title_color: Color,
    pub content_color: Color,
    pub tool_type: String,
}

pub enum MessageContent {
    Plain(String, Style),
    Styled(Line<'static>),
    StyledBlock(Vec<Line<'static>>),
    Markdown(String),
    PlainText(String),
    BashBubble {
        title: String,
        content: Vec<String>,
        colors: BubbleColors,
        tool_type: String,
    },
}

pub struct Message {
    pub id: Uuid,
    pub content: MessageContent,
}

impl Message {
    pub fn info(text: impl Into<String>, style: Option<Style>) -> Self {
        Message {
            id: Uuid::new_v4(),
            content: MessageContent::Plain(
                text.into(),
                style.unwrap_or(Style::default().fg(ratatui::style::Color::DarkGray)),
            ),
        }
    }
    pub fn user(text: impl Into<String>, style: Option<Style>) -> Self {
        Message {
            id: Uuid::new_v4(),
            content: MessageContent::Plain(
                text.into(),
                style.unwrap_or(Style::default().fg(ratatui::style::Color::Rgb(180, 180, 180))),
            ),
        }
    }
    pub fn assistant(id: Option<Uuid>, text: impl Into<String>, style: Option<Style>) -> Self {
        Message {
            id: id.unwrap_or(Uuid::new_v4()),
            content: MessageContent::Plain(text.into(), style.unwrap_or_default()),
        }
    }
    pub fn styled(line: Line<'static>) -> Self {
        Message {
            id: Uuid::new_v4(),
            content: MessageContent::Styled(line),
        }
    }
    pub fn markdown(text: impl Into<String>) -> Self {
        Message {
            id: Uuid::new_v4(),
            content: MessageContent::Markdown(text.into()),
        }
    }

    pub fn plain_text(text: impl Into<String>) -> Self {
        Message {
            id: Uuid::new_v4(),
            content: MessageContent::PlainText(text.into()),
        }
    }
}

pub fn get_wrapped_plain_lines<'a>(
    text: &'a str,
    style: &Style,
    width: usize,
) -> Vec<(Line<'a>, Style)> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut current = line;
        while !current.is_empty() {
            let take = current
                .char_indices()
                .scan(0, |acc, (i, c)| {
                    *acc += unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
                    Some((i, *acc))
                })
                .take_while(|&(_i, w)| w <= width)
                .last()
                .map(|(i, _w)| i + 1)
                .unwrap_or(current.len());
            if take == 0 {
                break;
            }
            let mut safe_take = take;
            while safe_take > 0 && !current.is_char_boundary(safe_take) {
                safe_take -= 1;
            }
            if safe_take == 0 {
                break;
            }
            let (part, rest) = current.split_at(safe_take);
            lines.push((Line::from(vec![Span::styled(part, *style)]), *style));
            current = rest;
        }
    }
    lines.push((Line::from(""), *style));
    lines
}

pub fn get_wrapped_styled_lines<'a>(line: &Line<'a>, _width: usize) -> Vec<(Line<'a>, Style)> {
    vec![
        (line.clone(), Style::default()),
        (Line::from(""), Style::default()),
    ]
}

pub fn get_wrapped_styled_block_lines<'a>(
    lines: &'a [Line<'a>],
    _width: usize,
) -> Vec<(Line<'a>, Style)> {
    lines
        .iter()
        .map(|l| (l.clone(), Style::default()))
        .collect()
}

pub fn get_wrapped_markdown_lines(markdown: &str, width: usize) -> Vec<(Line<'_>, Style)> {
    let mut result = Vec::new();
    let rendered_lines = render_markdown_to_lines(markdown, width);
    for line in rendered_lines {
        result.push((line, Style::default()));
    }
    result.push((Line::from(""), Style::default()));
    result
}

pub fn get_wrapped_bash_bubble_lines<'a>(
    _title: &'a str,
    content: &'a [String],
    colors: &BubbleColors,
) -> Vec<(Line<'a>, Style)> {
    let _title_style = Style::default()
        .fg(colors.title_color)
        .add_modifier(Modifier::BOLD);
    let border_style = Style::default().fg(colors.border_color);
    let content_style = Style::default().fg(colors.content_color);
    let mut lines = Vec::new();
    // lines.push((
    //     Line::from(vec![Span::styled(title, title_style)]),
    //     title_style,
    // ));
    for line in content.iter() {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() > 2 && chars[0] == '│' && chars[chars.len() - 1] == '│' {
            let mut spans = Vec::new();
            spans.push(Span::styled(chars[0].to_string(), border_style));
            let content: String = chars[1..chars.len() - 1].iter().collect();
            spans.push(Span::styled(content, content_style));
            spans.push(Span::styled(
                chars[chars.len() - 1].to_string(),
                border_style,
            ));
            lines.push((Line::from(spans), border_style));
        } else if line.starts_with('╭') || line.starts_with('╰') {
            lines.push((
                Line::from(vec![Span::styled(line.clone(), border_style)]),
                border_style,
            ));
        } else {
            lines.push((
                Line::from(vec![Span::styled(line.clone(), content_style)]),
                content_style,
            ));
        }
    }
    lines.push((Line::from(""), content_style));
    lines
}

pub fn get_wrapped_message_lines(messages: &[Message], width: usize) -> Vec<(Line<'_>, Style)> {
    let mut all_lines = Vec::new();
    for msg in messages {
        match &msg.content {
            MessageContent::Plain(text, style) => {
                all_lines.extend(get_wrapped_plain_lines(text, style, width));
            }
            MessageContent::Styled(line) => {
                all_lines.extend(get_wrapped_styled_lines(line, width));
            }
            MessageContent::StyledBlock(lines) => {
                all_lines.extend(get_wrapped_styled_block_lines(lines, width));
            }
            MessageContent::Markdown(markdown) => {
                all_lines.extend(get_wrapped_markdown_lines(markdown, width));
            }
            MessageContent::PlainText(text) => {
                all_lines.push((
                    Line::from(vec![Span::styled(text, Style::default())]),
                    Style::default(),
                ));
            }
            MessageContent::BashBubble {
                title,
                content,
                colors,
                tool_type: _,
            } => {
                all_lines.extend(get_wrapped_bash_bubble_lines(title, content, colors));
            }
        }
    }
    all_lines
}

pub fn extract_truncated_command_arguments(tool_call: &ToolCall) -> String {
    let arguments = serde_json::from_str::<Value>(&tool_call.function.arguments);
    const MAX_PARAMS: usize = 3;
    match arguments {
        Ok(Value::Object(obj)) => {
            let mut values = obj
                .into_iter()
                .map(|(key, val)| (key, format_simple_value(&val)))
                .collect::<Vec<_>>();
            values.sort_by_key(|(_, val)| val.len());
            let total_params = values.len();
            let included_params = values
                .into_iter()
                .take(MAX_PARAMS)
                .map(|(key, val)| {
                    if val.len() > 10 {
                        format!("{} = {:.20}...", key, val).replace("\n", " ")
                    } else {
                        format!("{} = {}", key, val)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            if total_params > MAX_PARAMS {
                format!("{}, {} more", included_params, total_params - MAX_PARAMS)
            } else {
                included_params
            }
        }
        _ => "unable to parse arguments".to_string(),
    }
}

pub fn extract_full_command_arguments(tool_call: &ToolCall) -> String {
    // First try to parse as valid JSON
    if let Ok(v) = serde_json::from_str::<Value>(&tool_call.function.arguments) {
        return format_json_value(&v);
    }

    // If JSON parsing fails, try regex patterns for malformed JSON
    let patterns = vec![
        // Pattern for key-value pairs with quotes
        r#"["']?(\w+)["']?\s*:\s*["']([^"']+)["']"#,
        // Pattern for simple key-value without quotes
        r#"(\w+)\s*:\s*([^,}\s]+)"#,
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            let mut results = Vec::new();
            for caps in re.captures_iter(&tool_call.function.arguments) {
                if caps.len() >= 3 {
                    let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let value = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    results.push(format!("{} = {}", key, value));
                }
            }
            if !results.is_empty() {
                return results.join(", ");
            }
        }
    }

    // Try to wrap in braces and parse as JSON
    let wrapped = format!("{{{}}}", tool_call.function.arguments);
    if let Ok(v) = serde_json::from_str::<Value>(&wrapped) {
        return format_json_value(&v);
    }

    // If all else fails, return the raw arguments if they're not empty
    let trimmed = tool_call.function.arguments.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    // Last resort
    format!("function_name={}", tool_call.function.name)
}

fn format_json_value(value: &Value) -> String {
    match value {
        Value::Object(obj) => {
            if obj.is_empty() {
                return "{}".to_string();
            }

            let mut values = obj
                .into_iter()
                .map(|(key, val)| (key, format_json_value(val)))
                .collect::<Vec<_>>();
            values.sort_by_key(|(_, val)| val.len());
            values
                .into_iter()
                .map(|(key, val)| {
                    if val.len() > 100 {
                        format!("{} = ```\n{}\n```", key, val)
                    } else {
                        format!("{} = {}", key, val)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                format!(
                    "[{}]",
                    arr.iter()
                        .map(format_simple_value)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        _ => format_simple_value(value),
    }
}

fn format_simple_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Object(_) => "object".to_string(),
        Value::Array(arr) => format!("[{}]", arr.len()),
    }
}

// Helper function to wrap text to specified width
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    if text.chars().count() <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        // If adding this word would exceed the width
        if current_width + word_len + (if current_width > 0 { 1 } else { 0 }) > width {
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
                current_width = 0;
            }

            // If a single word is longer than width, we need to break it
            if word_len > width {
                let mut remaining = word;
                while !remaining.is_empty() {
                    let chunk_size = std::cmp::min(width, remaining.chars().count());
                    let chunk: String = remaining.chars().take(chunk_size).collect();
                    lines.push(chunk.clone());
                    remaining = &remaining[chunk.len()..];
                }
                continue;
            }
        }

        if current_width > 0 {
            current_line.push(' ');
            current_width += 1;
        }

        current_line.push_str(word);
        current_width += word_len;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// Helper function to extract what the command is trying to do (bubble title)
pub fn extract_command_purpose(command: &str, outside_title: &str) -> String {
    let command = command.trim();

    // File creation patterns
    if let Some(pos) = command.find(" > ") {
        let after_redirect = &command[pos + 3..];
        if let Some(filename) = after_redirect.split_whitespace().next() {
            return format!("Creating {}", filename);
        }
    }

    if command.starts_with("cat >") {
        if let Some(after_cat) = command.strip_prefix("cat >") {
            if let Some(filename) = after_cat.split_whitespace().next() {
                return format!("Creating {}", filename);
            }
        }
    }

    if command.contains("echo") && command.contains(" > ") {
        if let Some(pos) = command.find(" > ") {
            let after_redirect = &command[pos + 3..];
            if let Some(filename) = after_redirect.split_whitespace().next() {
                return format!("Creating {}", filename);
            }
        }
    }

    if command.starts_with("touch ") {
        let after_touch = command.strip_prefix("touch ");
        if let Some(filename) = after_touch {
            if let Some(filename) = filename.split_whitespace().next() {
                return format!("Creating {}", filename);
            }
        }
    }

    if command.starts_with("mkdir ") {
        let after_mkdir = command.strip_prefix("mkdir ");
        if let Some(dirname) = after_mkdir {
            if let Some(dirname) = dirname.split_whitespace().next() {
                return format!("Creating directory {}", dirname);
            }
        }
    }

    if command.starts_with("rm ") {
        let after_rm = command.strip_prefix("rm ");
        if let Some(filename) = after_rm {
            if let Some(filename) = filename.split_whitespace().next() {
                return format!("Deleting {}", filename);
            }
        }
    }

    if command.starts_with("cp ") {
        return "Copying file".to_string();
    }

    if command.starts_with("mv ") {
        return "Moving file".to_string();
    }

    if command.starts_with("ls") {
        return "Listing directory".to_string();
    }

    if command.starts_with("cd ") {
        let after_cd = command.strip_prefix("cd ");
        if let Some(dirname) = after_cd {
            if let Some(dirname) = dirname.split_whitespace().next() {
                return format!("Changing to {}", dirname);
            }
        }
    }

    if command.starts_with("git ") {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() > 1 {
            match parts[1] {
                "add" => return "Adding files to git".to_string(),
                "commit" => return "Committing changes".to_string(),
                "push" => return "Pushing to remote".to_string(),
                "pull" => return "Pulling from remote".to_string(),
                "clone" => return "Cloning repository".to_string(),
                _ => return format!("Git {}", parts[1]),
            }
        }
    }

    if command.starts_with("npm ") {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() > 1 {
            match parts[1] {
                "install" => return "Installing npm packages".to_string(),
                "start" => return "Starting npm script".to_string(),
                "run" => return "Running npm script".to_string(),
                "build" => return "Building project".to_string(),
                _ => return format!("Running npm {}", parts[1]),
            }
        }
    }

    if command.starts_with("python ") || command.starts_with("python3 ") {
        return "Running Python script".to_string();
    }

    if command.starts_with("node ") {
        return "Running Node.js script".to_string();
    }

    if command.starts_with("cargo ") {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() > 1 {
            match parts[1] {
                "build" => return "Building Rust project".to_string(),
                "run" => return "Running Rust project".to_string(),
                "test" => return "Testing Rust project".to_string(),
                _ => return format!("Cargo {}", parts[1]),
            }
        }
    }

    // Default: return the command itself (first few words)
    let words: Vec<&str> = command.split_whitespace().take(3).collect();

    if words.is_empty() {
        "Running command".to_string()
    } else if !outside_title.is_empty() {
        return outside_title.to_string();
    } else {
        words.join(" ")
    }
}

// Helper function to get command name for the outside title
pub fn get_command_type_name(tool_call: &ToolCall) -> String {
    match tool_call.function.name.as_str() {
        "create_file" => "Create file".to_string(),
        "edit_file" => "Edit file".to_string(),
        "run_command" => "Run command".to_string(),
        "read_file" => "Read file".to_string(),
        "delete_file" => "Delete file".to_string(),
        "list_directory" => "List directory".to_string(),
        "search_files" => "Search files".to_string(),
        _ => {
            // Convert function name to title case
            tool_call
                .function
                .name
                .replace("_", " ")
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_various_formats() {
        // Test cases based on your examples
        let test_cases = vec![
            (r#"{"path":"."}"#, "path=."),
            (r#"{"confidence":1.0}"#, "confidence=1.0"),
            (r#"{"command":"ls -la"}"#, "command=ls -la"),
            (
                r#"{"action":"view","target":"file.txt"}"#,
                "action=view, target=file.txt",
            ),
            (r#"path: ".", mode: "list""#, "path=., mode=list"),
            ("", "function_name=test"),
        ];

        for (input, expected) in test_cases {
            let tool_call = ToolCall {
                id: "test".to_string(),
                r#type: "function".to_string(),
                function: FunctionCall {
                    name: "test".to_string(),
                    arguments: input.to_string(),
                },
            };

            let result = extract_full_command_arguments(&tool_call);
            println!(
                "Input: '{}' -> Output: '{}' (Expected: '{}')",
                input, result, expected
            );
        }
    }
}
