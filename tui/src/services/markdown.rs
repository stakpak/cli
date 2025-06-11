use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};

pub fn render_markdown_to_lines(markdown: &str, width: usize) -> Vec<Line<'static>> {
    let parser = Parser::new(markdown);
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut list_stack = Vec::new();
    let mut in_code_block = false;
    let mut _in_heading = false;
    let mut heading_level = 1;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading(level, _, _) => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    _in_heading = true;
                    heading_level = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                }
                Tag::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                Tag::CodeBlock(kind) => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    in_code_block = true;

                    match kind {
                        CodeBlockKind::Fenced(lang) => {
                            lines.push(Line::from(vec![
                                Span::styled("```", Style::default().fg(Color::DarkGray)),
                                Span::styled(lang.to_string(), Style::default().fg(Color::Green)),
                            ]));
                        }
                        CodeBlockKind::Indented => {
                            lines.push(Line::from(Span::styled(
                                "```",
                                Style::default().fg(Color::DarkGray),
                            )));
                        }
                    }
                }
                Tag::List(first_item_number) => {
                    list_stack.push(first_item_number);
                }
                Tag::Item => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }

                    let indent = " ".repeat((list_stack.len() - 1) * 2);
                    let bullet = if let Some(Some(num)) = list_stack.last() {
                        format!("{}. ", num)
                    } else {
                        "• ".to_string()
                    };

                    current_line.push(Span::styled(
                        format!("{}{}", indent, bullet),
                        Style::default().fg(Color::Gray),
                    ));
                }
                Tag::Strong => {}
                Tag::Emphasis => {}
                Tag::Link(_, _, _) => {}
                Tag::BlockQuote => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    current_line.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                Tag::Heading(_, _, _) => {
                    if !current_line.is_empty() {
                        let prefix = "#".repeat(heading_level) + " ";
                        let mut heading_line = vec![Span::styled(
                            prefix,
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        )];
                        heading_line.extend(current_line.drain(..).map(|mut span| {
                            span.style = span.style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                            span
                        }));
                        lines.push(Line::from(heading_line));
                        lines.push(Line::from("")); // Empty line after heading
                    }
                    _in_heading = false;
                }
                Tag::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    if !in_code_block {
                        lines.push(Line::from("")); // Empty line after paragraph
                    }
                }
                Tag::CodeBlock(_) => {
                    lines.push(Line::from(Span::styled(
                        "```",
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                    in_code_block = false;
                }
                Tag::List(_) => {
                    list_stack.pop();
                    if list_stack.is_empty() {
                        lines.push(Line::from("")); // Empty line after list
                    }
                }
                Tag::Item => {
                    if let Some(Some(num)) = list_stack.last_mut() {
                        *num += 1;
                    }
                }
                Tag::BlockQuote => {
                    lines.push(Line::from(""));
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default()
                                .fg(Color::Yellow)
                                .bg(Color::Rgb(40, 40, 40)),
                        )));
                    }
                } else {
                    current_line.push(Span::raw(text.to_string()));
                }
            }
            Event::Code(code) => {
                current_line.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
            }
            Event::Rule => {
                if !current_line.is_empty() {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
                lines.push(Line::from(Span::styled(
                    "─".repeat(width.min(80)),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    lines
}
