use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};
use tui_textarea::{Input, Key, TextArea};

use rgpt_types::message::{Message, Role};

// FIXME: hacky-ass functions
fn char_to_input(c: char) -> Input {
    fn enter() -> Input {
        Input {
            key: Key::Enter,
            ..Default::default()
        }
    }
    fn default(c: char, uppercase: bool) -> Input {
        Input {
            key: Key::Char(c),
            shift: uppercase,
            ..Default::default()
        }
    }
    match c {
        '\n' => enter(),
        c => default(c, false),
    }
}

// FIXME: hacky-ass functions
fn string_to_inputs(s: &str) -> Vec<Input> {
    s.chars().map(char_to_input).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAreaId {
    User,
    Assistant,
    System,
}

impl From<rgpt_types::message::Role> for SessionAreaId {
    fn from(id: rgpt_types::message::Role) -> Self {
        match id {
            Role::User => SessionAreaId::User,
            Role::Assistant => SessionAreaId::Assistant,
            Role::System => SessionAreaId::System,
        }
    }
}

impl From<SessionAreaId> for rgpt_types::message::Role {
    fn from(id: SessionAreaId) -> Self {
        match id {
            SessionAreaId::User => Role::User,
            SessionAreaId::Assistant => Role::Assistant,
            SessionAreaId::System => Role::System,
        }
    }
}

impl From<&str> for SessionAreaId {
    fn from(id: &str) -> Self {
        match id {
            "user" => SessionAreaId::User,
            "assistant" => SessionAreaId::Assistant,
            "system" => SessionAreaId::System,
            _ => SessionAreaId::User,
        }
    }
}

impl From<SessionAreaId> for String {
    fn from(id: SessionAreaId) -> Self {
        match id {
            SessionAreaId::User => "user".to_string(),
            SessionAreaId::Assistant => "assistant".to_string(),
            SessionAreaId::System => "system".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct SessionTextArea<'a> {
    pub id: SessionAreaId,
    pub title: String,
    pub text_area: TextArea<'a>,
    pub locked: bool,

    // FIXME: patch until tui-textarea implements wrapping.
    pub max_line_length: usize,
}

impl<'a> std::fmt::Debug for SessionTextArea<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTextArea")
            .field("id", &self.id)
            .finish()
    }
}

impl<'a> SessionTextArea<'a> {
    pub fn new(id: SessionAreaId, lines: &[&str], max_line_length: usize) -> Self {
        tracing::trace!("Creating new SessionTextArea with id: {:?}", id);
        let mut s = SessionTextArea {
            id,
            title: "temp".to_string(),
            text_area: Self::text_area_format(),
            max_line_length,
            locked: false,
        };
        if !lines.is_empty() {
            for input in string_to_inputs(lines.join("\n").as_str()) {
                s.input(input);
            }
            s.input(Input {
                key: Key::Enter,
                ..Default::default()
            });
        }
        s.inactivate();
        s
    }

    pub fn unlock(&mut self) {
        self.locked = false;
    }

    pub fn lock(&mut self) {
        self.locked = true;
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn set_title(&mut self, title: String) {
        tracing::trace!("Setting title for {:?} to: {}", self.id, title);
        self.title = title;
    }

    fn text_area_format() -> TextArea<'a> {
        let mut text_area = TextArea::default();
        text_area.set_cursor_line_style(Style::default());
        text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray)),
        );
        text_area
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    pub fn clear(&mut self) {
        self.text_area.select_all();
        self.text_area.cut();
        self.inactivate();
    }

    fn lines(&self) -> &[String] {
        self.text_area.lines()
    }

    pub fn message(&self) -> Option<Message> {
        if self.is_empty() {
            None
        } else {
            Some(Message {
                role: self.id.into(),
                content: self.lines().join("\n"),
            })
        }
    }

    pub fn set_message(&mut self, message: Message) {
        self.clear();
        for input in string_to_inputs(message.content.as_str()) {
            self.input(input);
        }
    }

    pub fn is_empty(&self) -> bool {
        let lines = self.text_area.lines();
        lines.is_empty() || lines.len() == 1 && (lines[0].is_empty() || lines[0] == "\n")
    }

    pub fn input(&mut self, input: Input) -> bool {
        match input.key {
            Key::Char(_) => {
                if self.is_locked() {
                    return false;
                }
                let current_line_length = self.lines().last().map_or(0, |l| l.len());
                if current_line_length + 1 >= self.max_line_length {
                    self.text_area.input(Input {
                        key: Key::Enter,
                        ..input
                    });
                }
                self.text_area.input(input)
            }
            Key::Backspace | Key::Delete | Key::Enter | Key::Tab => {
                if self.is_locked() {
                    return false;
                }
                self.text_area.input(input)
            }
            _ => self.text_area.input(input),
        };
        true
    }

    pub fn force_input(&mut self, input: Input) {
        self.locked = false;
        self.input(input);
        self.locked = true;
    }

    pub fn text_area(&self) -> &TextArea<'a> {
        &self.text_area
    }

    pub fn activate(&mut self) {
        tracing::trace!(
            "Activating SessionTextArea: {:?} with title {}",
            self.id,
            self.title()
        );
        self.text_area
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        self.text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .title(self.title()),
        );
    }

    pub fn inactivate(&mut self) {
        self.text_area.set_cursor_style(Style::default());
        self.text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray))
                .title(self.title()),
        );
    }
}

impl<'a> From<&'a SessionTextArea<'a>> for Message {
    fn from(text_area: &'a SessionTextArea<'a>) -> Self {
        Message {
            role: text_area.id.into(),
            content: text_area.lines().join("\n"),
        }
    }
}
