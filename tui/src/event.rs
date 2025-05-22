use crate::app::InputEvent;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind, MouseEvent, MouseButton};
use ratatui::layout::Position;

pub fn map_crossterm_event_to_input_event(event: Event) -> Option<InputEvent> {
    eprintln!("event: {:?}", event);
    match event {
        Event::Key(KeyEvent {
            code, modifiers, ..
        }) => match code {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                Some(InputEvent::Quit)
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::ALT) => {
                Some(InputEvent::CopySelection)
            }
            KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                Some(InputEvent::InputChangedNewline)
            }
            KeyCode::Char(c) => Some(InputEvent::InputChanged(c)),
            KeyCode::Backspace => Some(InputEvent::InputBackspace),
            KeyCode::Enter => Some(InputEvent::InputSubmitted),
            KeyCode::Esc => Some(InputEvent::HandleEsc),
            KeyCode::Up => Some(InputEvent::Up),
            KeyCode::Down => Some(InputEvent::Down),
            KeyCode::Left => Some(InputEvent::CursorLeft),
            KeyCode::Right => Some(InputEvent::CursorRight),
            KeyCode::PageUp => Some(InputEvent::PageUp),
            KeyCode::PageDown => Some(InputEvent::PageDown),
            _ => None,
        },
        
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            ..
        }) => Some(InputEvent::MouseDown(Position::new(column, row))),
        
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column,
            row,
            ..
        }) => Some(InputEvent::MouseDrag(Position::new(column, row))),
        
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column,
            row,
            ..
        }) => Some(InputEvent::MouseUp(Position::new(column, row))),
        
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            ..
        }) => Some(InputEvent::ScrollDown),
        
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            ..
        }) => Some(InputEvent::ScrollUp),
        Event::Resize(w, h) => Some(InputEvent::Resized(w, h)),
        _ => None,
    }
}
