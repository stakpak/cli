use super::message::{extract_full_command_arguments, extract_truncated_command_arguments};
use crate::app::AppState;
use crate::services::markdown::render_markdown_to_lines;
use crate::services::message::{
    BubbleColors, Message, MessageContent, extract_command_purpose, get_command_type_name,
    wrap_text,
};
use ratatui::layout::Size;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use stakpak_shared::models::integrations::openai::ToolCall;
use uuid::Uuid;

pub fn extract_bash_block_info(
    tool_call: &ToolCall,
    output: &str,
) -> (String, String, String, BubbleColors) {
    let full_command = extract_full_command_arguments(tool_call);
    let command = if full_command == "unknown command" {
        output.to_string()
    } else {
        full_command
    };
    let outside_title = get_command_type_name(tool_call);
    let bubble_title = extract_command_purpose(&command, &outside_title);
    let colors = match tool_call.function.name.as_str() {
        "create_file" => BubbleColors {
            border_color: Color::Green,
            title_color: Color::White,
            content_color: Color::LightGreen,
            tool_type: "create_file".to_string(),
        },
        "edit_file" => BubbleColors {
            border_color: Color::Yellow,
            title_color: Color::White,
            content_color: Color::LightYellow,
            tool_type: "edit_file".to_string(),
        },
        "run_command" => BubbleColors {
            border_color: Color::Cyan,
            title_color: Color::Yellow,
            content_color: Color::Gray,
            tool_type: "run_command".to_string(),
        },
        "read_file" => BubbleColors {
            border_color: Color::Magenta,
            title_color: Color::White,
            content_color: Color::LightMagenta,
            tool_type: "read_file".to_string(),
        },
        "delete_file" => BubbleColors {
            border_color: Color::Red,
            title_color: Color::White,
            content_color: Color::LightRed,
            tool_type: "delete_file".to_string(),
        },
        _ => BubbleColors {
            border_color: Color::Cyan,
            title_color: Color::White,
            content_color: Color::Gray,
            tool_type: "unknown".to_string(),
        },
    };
    (command, outside_title, bubble_title, colors)
}

fn create_bordered_content_lines(
    content_lines: &[Line<'static>],
    bubble_title: &str,
    inner_width: usize,
    border_color: Color,
) -> Vec<Line<'static>> {
    let mut bordered = Vec::new();
    let border_style = Style::default().fg(border_color);

    // Top border
    let title_width = bubble_title.chars().count();
    let horizontal_line = "─".repeat(inner_width + 2);
    let top = if title_width <= inner_width {
        let remaining_dashes = inner_width + 2 - title_width;
        format!("╭{}{}", bubble_title, "─".repeat(remaining_dashes)) + "╮"
    } else {
        let truncated_title = bubble_title.chars().take(inner_width).collect::<String>();
        format!("╭{}─╮", truncated_title)
    };
    bordered.push(Line::from(vec![Span::styled(top, border_style)]));

    // Content lines with borders
    for line in content_lines {
        let line_width = line.width();
        let padding_needed = if inner_width > line_width {
            inner_width - line_width
        } else {
            0
        };
        let mut spans = Vec::new();
        spans.push(Span::styled("│ ", border_style));
        spans.extend(line.spans.clone()); // <-- preserve all original styles!
        if padding_needed > 0 {
            spans.push(Span::raw(" ".repeat(padding_needed)));
        }
        spans.push(Span::styled(" │", border_style));
        bordered.push(Line::from(spans));
    }

    // Bottom border
    let bottom = format!("╰{}╯", horizontal_line);
    bordered.push(Line::from(vec![Span::styled(bottom, border_style)]));
    bordered
}

fn is_md_block(content: &str) -> bool {
    if content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("path =") && trimmed.ends_with(".md")
            || trimmed.starts_with("file =") && trimmed.ends_with(".md")
            || trimmed.starts_with("filename =") && trimmed.ends_with(".md")
    }) {
        return true;
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub fn render_styled_block(
    content: &str,
    outside_title: &str,
    bubble_title: &str,
    colors: Option<BubbleColors>,
    state: &mut AppState,
    terminal_size: Size,
    tool_type: &str,
    message_id: Option<Uuid>,
) -> Uuid {
    let is_md = is_md_block(content);

    if is_md {
        let terminal_width = terminal_size.width as usize;
        let content_width = if terminal_width > 4 {
            terminal_width - 4
        } else {
            40
        };
        let inner_width = content_width;
        let processed = render_markdown_to_lines(content, inner_width);
        let colors = colors.clone().unwrap_or(BubbleColors {
            border_color: Color::Blue,
            title_color: Color::White,
            content_color: Color::Reset,
            tool_type: "markdown".to_string(),
        });
        let bordered_content = create_bordered_content_lines(
            &processed,
            bubble_title,
            inner_width,
            colors.border_color,
        );
        let message_id = message_id.unwrap_or_else(Uuid::new_v4);
        state.messages.push(Message {
            id: message_id,
            content: MessageContent::BashBubble {
                title: outside_title.to_string(),
                content: bordered_content,
                colors,
                tool_type: "markdown".to_string(),
            },
        });
        return message_id;
    }

    let terminal_width = terminal_size.width as usize;
    let content_width = if terminal_width > 4 {
        terminal_width - 4
    } else {
        40
    };

    let content_lines = content.split('\n').collect::<Vec<_>>();

    let inner_width = content_width;
    let horizontal_line = "─".repeat(inner_width + 2);
    let bottom_border = format!("╰{}╯", horizontal_line);
    let title_border = {
        let title_width = bubble_title.chars().count();
        if title_width <= inner_width {
            let remaining_dashes = inner_width + 2 - title_width;
            format!("╭{}{}", bubble_title, "─".repeat(remaining_dashes)) + "╮"
        } else {
            let truncated_title = bubble_title.chars().take(inner_width).collect::<String>();
            format!("╭{}─╮", truncated_title)
        }
    };
    let mut bubble_lines = vec![];

    bubble_lines.push(title_border);
    for line in &content_lines {
        let trimmed_line = line.trim_end();
        if trimmed_line.is_empty() {
            let padding = " ".repeat(inner_width);
            bubble_lines.push(format!("│ {} │", padding));
            continue;
        }
        let wrapped_lines = wrap_text(trimmed_line, inner_width);
        for wrapped_line in wrapped_lines {
            let line_char_count = wrapped_line.chars().count();
            let padding_needed = inner_width - line_char_count;
            let padding = " ".repeat(padding_needed);
            let formatted_line = format!("│ {}{} │", wrapped_line, padding);
            bubble_lines.push(formatted_line);
        }
    }
    bubble_lines.push(bottom_border);

    let default_colors = BubbleColors {
        border_color: Color::Cyan,
        title_color: Color::White,
        content_color: Color::Gray,
        tool_type: "unknown".to_string(),
    };

    let message_id = message_id.unwrap_or_else(Uuid::new_v4);
    state.messages.push(Message {
        id: message_id,
        content: MessageContent::BashBubble {
            title: outside_title.to_string(),
            content: bubble_lines.into_iter().map(Line::from).collect(),
            colors: colors.clone().unwrap_or(default_colors),
            tool_type: tool_type.to_string(),
        },
    });
    message_id
}

pub fn render_bash_block(
    tool_call: &ToolCall,
    output: &str,
    _accepted: bool,
    state: &mut AppState,
    terminal_size: Size,
) -> Uuid {
    let (command, outside_title, bubble_title, colors) = extract_bash_block_info(tool_call, output);
    render_styled_block(
        &command,
        &outside_title,
        &bubble_title,
        Some(colors.clone()),
        state,
        terminal_size,
        &tool_call.function.name,
        None,
    )
}

pub fn render_result_block(tool_call: &ToolCall, result: &str, state: &mut AppState) {
    let mut lines = Vec::new();
    // Header line with approved colors (green bullet, white text)
    lines.push(Line::from(vec![
        Span::styled(
            "● ",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            tool_call.function.name.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({})", extract_truncated_command_arguments(tool_call)),
            Style::default().fg(Color::Gray),
        ),
    ]));

    // Show the command output
    let output_pad = "    "; // 4 spaces for indentation
    for (i, line) in result.lines().enumerate() {
        let prefix = if i == 0 { "└ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{output_pad}{prefix}"),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(line, Style::default().fg(Color::Gray)),
        ]));
    }

    let mut owned_lines: Vec<Line<'static>> = lines
        .into_iter()
        .map(|line| {
            let owned_spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            Line::from(owned_spans)
        })
        .collect();
    owned_lines.push(Line::from(""));

    state.messages.push(Message {
        id: Uuid::new_v4(),
        content: MessageContent::StyledBlock(owned_lines),
    });
}

// Function to render a rejected bash command (when user selects "No")
pub fn render_bash_block_rejected(command_name: &str, state: &mut AppState) {
    let mut lines = Vec::new();

    // Header - similar to regular bash block
    lines.push(Line::from(vec![
        Span::styled(
            "● ",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Bash",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({})", command_name),
            Style::default().fg(Color::Gray),
        ),
        Span::styled("...", Style::default().fg(Color::Gray)),
    ]));

    // Add the rejection line
    lines.push(Line::from(vec![Span::styled(
        "  L No (tell Stakpak what to do differently)",
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )]));

    let mut owned_lines: Vec<Line<'static>> = lines
        .into_iter()
        .map(|line| {
            let owned_spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            Line::from(owned_spans)
        })
        .collect();
    owned_lines.push(Line::from(""));

    state.messages.push(Message {
        id: Uuid::new_v4(),
        content: MessageContent::StyledBlock(owned_lines),
    });
}
