use crate::app::AppState;
use crate::services::confirmation_dialog::render_confirmation_dialog;
use crate::services::helper_block::render_loading_spinner;
use crate::services::helper_dropdown::render_helper_dropdown;
use crate::services::hint_helper::render_hint_or_shortcuts;
use crate::services::message::get_wrapped_message_lines;
use crate::services::message_pattern::{
    process_agent_mode_patterns, process_checkpoint_patterns, process_section_title_patterns,
    spans_to_string,
};
use crate::services::sessions_dialog::render_sessions_dialog;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn view(f: &mut Frame, state: &AppState) {
    // Calculate the required height for the input area based on content
    let input_area_width = f.area().width.saturating_sub(4) as usize;
    let input_lines = calculate_input_lines(&state.input, input_area_width); // -4 for borders and padding
    let input_height = if state.is_dialog_open {
        input_lines as u16
    } else {
        (input_lines + 2) as u16
    };

    let margin_height = 2;
    let dropdown_showing = state.show_helper_dropdown
        && !state.filtered_helpers.is_empty()
        && state.input.starts_with('/');
    let dropdown_height = if dropdown_showing {
        state.filtered_helpers.len() as u16
    } else {
        0
    };
    let hint_height = if dropdown_showing { 0 } else { margin_height };

    let dialog_height = if state.show_sessions_dialog { 11 } else { 0 };
    let dialog_margin = if state.is_dialog_open || state.show_sessions_dialog {
        1
    } else {
        0
    };

    // Layout: [messages][dialog_margin][dialog][input][dropdown][hint]
    let mut constraints = vec![
        Constraint::Min(1), // messages
        Constraint::Length(dialog_margin),
        Constraint::Length(dialog_height),
    ];
    if !state.show_sessions_dialog {
        constraints.push(Constraint::Length(input_height));
        constraints.push(Constraint::Length(dropdown_height));
        constraints.push(Constraint::Length(hint_height));
    }
    let chunks = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let message_area = chunks[0];
    let mut input_area = Rect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    let mut dropdown_area = Rect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    let mut hint_area = Rect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    if !state.show_sessions_dialog {
        input_area = chunks[3];
        dropdown_area = chunks.get(4).copied().unwrap_or(input_area);
        hint_area = chunks.get(5).copied().unwrap_or(input_area);
    }
    let message_area_width = message_area.width as usize;
    let message_area_height = message_area.height as usize;

    render_messages(
        f,
        state,
        message_area,
        message_area_width,
        message_area_height,
    );

    if state.is_dialog_open {
        render_confirmation_dialog(f, state);
    }

    // Only render input, dropdown, and hint if dialog is not open and sessions dialog is not open
    if !state.is_dialog_open && !state.show_sessions_dialog {
        render_multiline_input(f, state, input_area);
        render_helper_dropdown(f, state, dropdown_area);
        if !dropdown_showing {
            render_hint_or_shortcuts(f, state, hint_area);
        }
    }
    if state.show_sessions_dialog {
        render_sessions_dialog(f, state);
    }
}

// Calculate how many lines the input will take up when wrapped
fn calculate_input_lines(input: &str, width: usize) -> usize {
    if input.is_empty() {
        return 1; // At least one line
    }
    let prompt_width = 2; // "> " prefix
    let first_line_width = width.saturating_sub(prompt_width);
    let available_width = width;
    if available_width <= 1 {
        return input.len(); // Fallback if width is too small
    }

    // Split by explicit newlines first
    let mut total_lines = 0;
    for line in input.split('\n') {
        // For each line segment after splitting by newlines
        let mut words = line.split_whitespace().peekable();
        let mut current_width = 0;
        let mut is_first_line_in_segment = true;

        while words.peek().is_some() {
            let word = words.next().unwrap_or_default();
            let word_width = word
                .chars()
                .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1))
                .sum::<usize>();

            // Determine available width for this line
            let line_width_limit = if is_first_line_in_segment && total_lines == 0 {
                first_line_width
            } else {
                available_width
            };

            // Add space before word (except at start of line)
            if current_width > 0 {
                current_width += 1; // Space width
            }

            // Check if word fits on current line
            if current_width + word_width <= line_width_limit {
                current_width += word_width;
            } else {
                // Word doesn't fit, start new line
                total_lines += 1;
                current_width = word_width;
                is_first_line_in_segment = false;
            }
        }

        total_lines += 1;
    }

    total_lines
}

fn render_messages(f: &mut Frame, state: &AppState, area: Rect, width: usize, height: usize) {
    let mut all_lines: Vec<(Line, Style)> = get_wrapped_message_lines(&state.messages, width);
    if state.loading {
        let loading_line = render_loading_spinner(state);
        all_lines.push((loading_line, Style::default()));
    }
    let total_lines = all_lines.len();
    let max_scroll = total_lines.saturating_sub(height);

    let scroll = if state.stay_at_bottom {
        max_scroll
    } else {
        state.scroll.min(max_scroll)
    };

    let mut visible_lines = Vec::new();
    let mut lines_added = 0;

    // Process only the lines we need, and ensure we don't exceed height
    for i in 0..height {
        if lines_added >= height {
            break; // Prevent overflow
        }

        if let Some((line, _)) = all_lines.get(scroll + i) {
            let line_text = spans_to_string(line);

            if line_text.contains("<checkpoint_id>") {
                let processed = process_checkpoint_patterns(
                    &[(line.clone(), Style::default())],
                    f.area().width as usize,
                );
                // Add processed lines but respect height limit
                for (processed_line, _) in processed {
                    if lines_added < height {
                        visible_lines.push(processed_line);
                        lines_added += 1;
                    }
                }
            } else if line_text.contains("<agent_mode>") {
                let processed = process_agent_mode_patterns(&[(line.clone(), Style::default())]);
                // Add processed lines but respect height limit
                for (processed_line, _) in processed {
                    if lines_added < height {
                        visible_lines.push(processed_line);
                        lines_added += 1;
                    }
                }
            } else {
                let section_tags = [
                    "planning",
                    "reasoning",
                    "notes",
                    "progress",
                    "local_context",
                ];
                let mut found = false;

                for tag in &section_tags {
                    let closing_tag = format!("</{}>", tag);
                    if line_text.trim() == closing_tag {
                        // Skip this line entirely
                        found = true;
                        break;
                    }
                    if line_text.contains(&format!("<{}>", tag)) {
                        let processed = process_section_title_patterns(
                            &[(line.clone(), Style::default())],
                            tag,
                        );
                        // Add processed lines but respect height limit
                        for (processed_line, _) in processed {
                            if lines_added < height {
                                visible_lines.push(processed_line);
                                lines_added += 1;
                            }
                        }
                        found = true;
                        break;
                    }
                }

                if !found && lines_added < height {
                    visible_lines.push(line.clone());
                    lines_added += 1;
                }
            }
        } else if lines_added < height {
            visible_lines.push(Line::from(""));
            lines_added += 1;
        }
    }

    // Ensure we don't exceed the allocated height
    visible_lines.truncate(height);

    let message_widget = Paragraph::new(visible_lines).wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(message_widget, area);
}

fn render_multiline_input(f: &mut Frame, state: &AppState, area: Rect) {
    // Mask input if in shell mode and waiting for shell input (password)
    let input = if state.show_shell_mode && state.waiting_for_shell_input {
        "*".repeat(state.input.chars().count())
    } else {
        state.input.clone()
    };
    let available_width = area.width.saturating_sub(4) as usize; // -4 for borders and padding

    // Ensure the cursor position is valid
    let cursor_pos = state.cursor_position.min(input.len());

    // Split the input by newlines first
    let line_segments: Vec<&str> = input.split('\n').collect();

    let mut lines = Vec::new();
    let mut cursor_rendered = false;

    // Track position in the input string (in bytes)
    let mut current_pos = 0;

    for (segment_idx, segment) in line_segments.iter().enumerate() {
        let mut current_line = Vec::new();
        // Add prompt to first line only
        let prompt = if segment_idx == 0 { "> " } else { "" };
        let prompt_width = prompt.len();
        current_line.push(Span::raw(prompt));

        let mut current_width = prompt_width;

        // Process this line segment
        let mut word_segments = Vec::new();
        let mut current_word = String::new();
        let mut in_word = false;

        // Split segment into words and spaces, preserving exact positions
        for (i, c) in segment.char_indices() {
            let byte_pos = current_pos + i;

            // Render cursor if it's at this exact position
            if byte_pos == cursor_pos && !cursor_rendered {
                if in_word {
                    // End current word before cursor
                    if !current_word.is_empty() {
                        word_segments.push((current_word.clone(), false));
                        current_word.clear();
                    }
                }

                // Add the cursor
                word_segments.push((c.to_string(), true));
                cursor_rendered = true;
                in_word = !c.is_whitespace();
            } else if c.is_whitespace() {
                // End current word if any
                if in_word && !current_word.is_empty() {
                    word_segments.push((current_word.clone(), false));
                    current_word.clear();
                    in_word = false;
                }

                // Add the whitespace
                word_segments.push((c.to_string(), false));
            } else {
                // Part of a word
                current_word.push(c);
                in_word = true;
            }
        }

        // Add any remaining word
        if in_word && !current_word.is_empty() {
            word_segments.push((current_word, false));
        }

        // If cursor is at the end of this segment
        if current_pos + segment.len() == cursor_pos && !cursor_rendered {
            word_segments.push((" ".to_string(), true));
            cursor_rendered = true;
        }

        // Render the word segments with proper wrapping
        for (text, is_cursor) in word_segments {
            let text_width = text
                .chars()
                .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1))
                .sum::<usize>();

            // Check if this segment would exceed line width
            let needs_wrap = !text.trim().is_empty()
                && current_width > prompt_width
                && current_width + text_width > available_width;

            if needs_wrap {
                // Add current line and start a new one
                lines.push(Line::from(std::mem::take(&mut current_line)));
                current_line = Vec::new();
                current_width = 0;
            }

            // Add the segment (with or without cursor highlighting)
            if is_cursor {
                current_line.push(Span::styled(
                    text,
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                current_line.push(Span::raw(text));
            }

            current_width += text_width;
        }

        // Add this line
        lines.push(Line::from(std::mem::take(&mut current_line)));

        // Move to next segment
        current_pos += segment.len() + 1; // +1 for newline
    }

    // If cursor is at the very end and we haven't rendered it yet
    if cursor_pos == input.len() && !cursor_rendered {
        // If the last line is empty, add cursor there
        if let Some(last_line) = lines.last_mut() {
            last_line.spans.push(Span::styled(
                " ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            // Create a new line with prompt and cursor
            lines.push(Line::from(vec![
                Span::raw("> "),
                Span::styled(
                    " ",
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    // Ensure we have at least one line
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("> "),
            Span::styled(
                " ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if state.show_shell_mode {
            Style::default().fg(Color::Rgb(160, 92, 158))
        } else {
            Style::default().fg(Color::DarkGray)
        });

    // Render the input widget
    let input_widget = Paragraph::new(lines)
        .style(Style::default())
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(input_widget, area);
}
