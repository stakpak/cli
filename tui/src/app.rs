use crate::services::helper_block::push_error_message;
use crate::services::message::Message;
use ratatui::style::Style;
use stakpak_shared::models::integrations::openai::{ToolCall, ToolCallResult};
use uuid::Uuid;

pub struct SessionInfo {
    pub title: String,
    pub id: String,
    pub updated_at: String,
}

// TODO: add user list sessions
pub fn list_sessions() -> Vec<SessionInfo> {
    vec![]
}

pub struct AppState {
    pub input: String,
    pub cursor_position: usize,
    pub cursor_visible: bool,
    pub messages: Vec<Message>,
    pub scroll: usize,
    pub scroll_to_bottom: bool,
    pub stay_at_bottom: bool,
    pub helpers: Vec<&'static str>,
    pub show_helper_dropdown: bool,
    pub helper_selected: usize,
    pub filtered_helpers: Vec<&'static str>,
    pub show_shortcuts: bool,
    pub is_dialog_open: bool,
    pub dialog_command: Option<ToolCall>,
    pub dialog_selected: usize,
    pub loading: bool,
    pub spinner_frame: usize,
    pub sessions: Vec<SessionInfo>,
    pub show_sessions_dialog: bool,
    pub session_selected: usize,
    pub account_info: String,
    pub pending_bash_message_id: Option<Uuid>, // New field to track pending bash message
}

#[derive(Debug)]
pub enum InputEvent {
    AssistantMessage(String),
    StreamAssistantMessage(Uuid, String),
    RunToolCall(ToolCall),
    ToolResult(ToolCallResult),
    Loading(bool),
    InputChanged(char),
    GetStatus(String),
    Error(String),
    InputBackspace,
    InputChangedNewline,
    InputSubmitted,
    InputSubmittedWith(String),
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    DropdownUp,
    DropdownDown,
    DialogUp,
    DialogDown,
    Up,
    Down,
    Quit,
    HandleEsc,
    CursorLeft,
    CursorRight,
    ToggleCursorVisible,
    Resized(u16, u16),
    ShowConfirmationDialog(ToolCall),
    DialogConfirm,
    DialogCancel,
    Tab,
}

#[derive(Debug)]
pub enum OutputEvent {
    UserMessage(String),
    AcceptTool(ToolCall),
    RejectTool(ToolCall),
}

impl AppState {
    pub fn new(helpers: Vec<&'static str>) -> Self {
        let mut state = AppState {
            input: String::new(),
            cursor_position: 0,
            cursor_visible: true,
            messages: vec![
                Message::info(
                    r"
 ▗▄▄▖▗▄▄▄▖▗▄▖ ▗▖ ▗▖▗▄▄▖  ▗▄▖ ▗▖ ▗▖     ▗▄▖  ▗▄▄▖▗▄▄▄▖▗▖  ▗▖▗▄▄▄▖
▐▌     █ ▐▌ ▐▌▐▌▗▞▘▐▌ ▐▌▐▌ ▐▌▐▌▗▞▘    ▐▌ ▐▌▐▌   ▐▌   ▐▛▚▖▐▌  █  
 ▝▀▚▖  █ ▐▛▀▜▌▐▛▚▖ ▐▛▀▘ ▐▛▀▜▌▐▛▚▖     ▐▛▀▜▌▐▌▝▜▌▐▛▀▀▘▐▌ ▝▜▌  █  
▗▄▄▞▘  █ ▐▌ ▐▌▐▌ ▐▌▐▌   ▐▌ ▐▌▐▌ ▐▌    ▐▌ ▐▌▝▚▄▞▘▐▙▄▄▖▐▌  ▐▌  █  ",
                    Some(Style::default().fg(ratatui::style::Color::Cyan)),
                ),
                Message::info("/help for help, /status for your current setup", None),
                Message::info(
                    format!(
                        "cwd: {}",
                        std::env::current_dir().unwrap_or_default().display()
                    ),
                    None,
                ),
            ],
            scroll: 0,
            scroll_to_bottom: false,
            stay_at_bottom: true,
            helpers: helpers.clone(),
            show_helper_dropdown: false,
            helper_selected: 0,
            filtered_helpers: helpers,
            show_shortcuts: false,
            is_dialog_open: false,
            dialog_command: None,
            dialog_selected: 0,
            loading: false,
            spinner_frame: 0,
            sessions: list_sessions(),
            show_sessions_dialog: false,
            session_selected: 0,
            account_info: String::new(),
            pending_bash_message_id: None, // Initialize new field
        };
        if std::env::current_dir().is_err() {
            push_error_message(&mut state, "Failed to get current working directory");
        }
        state
    }
}
