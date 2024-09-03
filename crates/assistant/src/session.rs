use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::stream::StreamExt;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::Terminal;
use ratatui::{backend::CrosstermBackend, layout::Rect};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use std::{io::stdout, rc::Rc};
use tui_textarea::{Input, Key, TextArea};

use crate::{
    error::Error,
    pagetree::{PageTree, PageTreeNodeId},
    Assistant,
};
use rgpt_types::{
    completion::TextEvent,
    message::{Message, Role},
};

pub struct Session {
    inner: SessionInner,
}

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

impl Session {
    pub fn setup(assistant: Assistant) -> Result<Self, Error> {
        Ok(Session {
            inner: SessionInner::new(assistant),
        })
    }

    pub async fn start(&mut self, messages: &[Message]) -> Result<(), Error> {
        self.inner.run(messages).await?;
        Ok(())
    }
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

#[derive(Debug, Clone)]
pub struct SessionTextArea<'a> {
    pub id: SessionAreaId,
    pub text_area: TextArea<'a>,

    // FIXME: patch until tui-textarea implements wrapping.
    pub max_line_length: usize,
}

impl<'a> SessionTextArea<'a> {
    pub fn new(id: SessionAreaId, lines: &[&str], max_line_length: usize) -> Self {
        let mut s = SessionTextArea {
            id,
            text_area: TextArea::default(),
            max_line_length,
        };
        s.text_area.set_cursor_line_style(Style::default());
        for input in string_to_inputs(lines.join("\n").as_str()) {
            s.input(input);
        }
        s.input(Input {
            key: Key::Enter,
            ..Default::default()
        });
        s.inactivate();
        s
    }

    fn title(&self) -> String {
        format!("{}: {}", String::from(self.id), "temp")
    }

    fn clear(&mut self) {
        self.text_area = TextArea::default();
        self.text_area.set_cursor_line_style(Style::default());
        self.inactivate();
    }

    fn lines(&self) -> &[String] {
        self.text_area.lines()
    }

    fn is_empty(&self) -> bool {
        let lines = self.text_area.lines();
        lines.is_empty() || lines.len() == 1 && lines[0].is_empty()
    }

    fn input(&mut self, input: Input) {
        match input.key {
            Key::Char(_) => {
                let current_line_length = self.lines().last().map_or(0, |l| l.len());
                if current_line_length + 1 >= self.max_line_length {
                    self.text_area.input(Input {
                        key: Key::Enter,
                        ..input
                    });
                }
                self.text_area.input(input)
            }
            _ => self.text_area.input(input),
        };
    }

    fn text_area(&self) -> &TextArea<'a> {
        &self.text_area
    }

    pub fn activate(&mut self) {
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

pub struct SessionLayout<'a> {
    pub page_tree: PageTree<'a>,
    pub current_node: PageTreeNodeId,

    pub system_area: SessionTextArea<'a>,

    pub active: SessionAreaId,

    // FIXME: patch until tui-textarea implements wrapping.
    pub max_line_length: usize,
}

impl<'a> SessionLayout<'a> {
    fn new(messages: &[Message]) -> Self {
        tracing::trace!("messages: {:?}", messages);
        // FIXME: patch until tui-textarea implements wrapping.
        let max_line_length = crossterm::terminal::size()
            .map(|(w, _)| (w.saturating_sub(10)) as usize / 2)
            .unwrap_or(70);
        tracing::trace!("max_line_length: {}", max_line_length);

        let text_areas = messages
            .iter()
            .map(|m| {
                let id = SessionAreaId::from(m.role);
                let lines = m.content.lines().collect::<Vec<_>>();
                SessionTextArea::new(id, lines.as_slice(), max_line_length)
            })
            .chain(std::iter::once(SessionTextArea::new(
                SessionAreaId::User,
                &[],
                max_line_length,
            )))
            .chain(std::iter::once(SessionTextArea::new(
                SessionAreaId::Assistant,
                &[],
                max_line_length,
            )))
            .collect::<Vec<_>>();

        let mut page_tree = PageTree::new();
        let current_node = page_tree
            .insert_text_areas(None, text_areas)
            .unwrap_or(page_tree.root_id());

        let system_area = SessionTextArea::new(SessionAreaId::System, &[], max_line_length);

        let active = SessionAreaId::User;
        let mut layout = SessionLayout {
            page_tree,
            current_node,
            system_area,
            active,
            max_line_length,
        };
        layout.current_node_area_mut(SessionAreaId::User).activate();
        layout
    }
    fn chunks(&self, chunk: Rect) -> (Rc<[Rect]>, Rc<[Rect]>) {
        let outer_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunk);

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
            .split(outer_layout[0]);
        (outer_layout, inner_layout)
    }

    fn current_node_area(&self, id: SessionAreaId) -> &SessionTextArea<'a> {
        match id {
            SessionAreaId::System => &self.system_area,
            _ => self.page_tree.get(self.current_node).unwrap().area(id),
        }
    }

    fn parent_node_area(&self, id: SessionAreaId) -> &SessionTextArea<'a> {
        match id {
            SessionAreaId::System => &self.system_area,
            _ => {
                let parent_id = self.page_tree.get(self.current_node).unwrap().parent;
                self.page_tree.get(parent_id).unwrap().area(id)
            }
        }
    }

    fn current_node_area_mut(&mut self, id: SessionAreaId) -> &mut SessionTextArea<'a> {
        match id {
            SessionAreaId::System => &mut self.system_area,
            _ => self
                .page_tree
                .get_mut(self.current_node)
                .unwrap()
                .area_mut(id),
        }
    }

    fn activate(&mut self, id: SessionAreaId) {
        if self.active == SessionAreaId::System {
            self.system_area.inactivate();
        }
        if id == SessionAreaId::System {
            self.system_area.activate();
        }
        self.page_tree.activate(self.current_node, id);
        self.active = id;
    }

    fn switch(&mut self) {
        self.activate(match self.active {
            SessionAreaId::User => SessionAreaId::Assistant,
            SessionAreaId::Assistant => SessionAreaId::System,
            SessionAreaId::System => SessionAreaId::User,
        });
    }

    fn input(&mut self, input: Input) {
        self.current_node_area_mut(self.active).input(input);
    }

    fn draw(&self, f: &mut Frame) {
        let (outer_layout, user_layout) = self.chunks(f.area());
        let user_area = self.current_node_area(SessionAreaId::User);
        let assistant_area = match self.current_node_area(SessionAreaId::Assistant) {
            area if area.is_empty() => self.parent_node_area(SessionAreaId::Assistant),
            area => area,
        };
        let system_area = self.current_node_area(SessionAreaId::System);
        f.render_widget(user_area.text_area(), user_layout[1]);
        f.render_widget(assistant_area.text_area(), outer_layout[1]);
        f.render_widget(system_area.text_area(), user_layout[0]);
    }

    fn messages(&self) -> Vec<Message> {
        let mut messages = vec![Message::from(self.current_node_area(SessionAreaId::System))];
        messages.extend(self.page_tree.collect_messages(self.current_node, None));
        messages
    }

    fn switch_node(&mut self, node: PageTreeNodeId) -> Option<PageTreeNodeId> {
        self.current_node = node;
        self.activate(self.active);
        Some(node)
    }

    fn down_one(&mut self) -> Option<PageTreeNodeId> {
        self.switch_node(self.page_tree.children(self.current_node).first()?.id)
    }

    fn up_one(&mut self) -> Option<PageTreeNodeId> {
        self.switch_node(self.page_tree.parent(self.current_node)?.id)
    }

    fn next_branch(&mut self) -> Option<PageTreeNodeId> {
        self.switch_node(self.page_tree.next_sibling(self.current_node)?.id)
    }

    fn previous_branch(&mut self) -> Option<PageTreeNodeId> {
        self.switch_node(self.page_tree.previous_sibling(self.current_node)?.id)
    }

    fn new_leaf(&mut self) {
        let id = self.page_tree.insert_child(self.current_node);
        self.current_node = id;
    }

    fn update(&mut self, messages: &[Message], node: Option<PageTreeNodeId>) -> Result<(), Error> {
        for message in messages {
            if message.role == Role::System {
                self.system_area.text_area = TextArea::from(message.content.lines());
            }
        }

        let text_areas = messages.iter().filter(|m| m.role != Role::System).map(|m| {
            let id = SessionAreaId::from(m.role);
            let lines = m.content.lines().collect::<Vec<_>>();
            SessionTextArea::new(id, lines.as_slice(), self.max_line_length)
        }).collect::<Vec<_>>();

        let id = self.page_tree.insert_text_areas(node, text_areas)?;

        self.switch_node(id);
        Ok(())
    }

    async fn handle_assistant_event(&mut self, event: TextEvent) {
        tracing::trace!("handling assistant stream");
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
        fn string_to_inputs(s: &str) -> Vec<Input> {
            s.chars().map(char_to_input).collect()
        }
        let area = self.current_node_area_mut(SessionAreaId::Assistant);
        match event {
            TextEvent::Null => {}
            TextEvent::MessageStart { .. } => {
                // clear the assistant buffer
                area.clear();
            }
            TextEvent::ContentBlockStart { content_block, .. } => {
                for input in string_to_inputs(content_block.text().unwrap_or_default().as_str()) {
                    area.input(input);
                }
            }
            TextEvent::ContentBlockDelta { delta, .. } => {
                for input in string_to_inputs(delta.text().unwrap_or_default().as_str()) {
                    area.input(input);
                }
            }
            TextEvent::ContentBlockStop { .. } => {}
            TextEvent::MessageDelta { .. } => {}
            TextEvent::MessageStop => {
                tracing::trace!("message stop");
                self.new_leaf();
            }
        }
        tracing::trace!("finished")
    }
}

pub struct SessionInner {
    assistant: Assistant,
    layout: SessionLayout<'static>,
}

impl SessionInner {
    fn new(assistant: Assistant) -> Self {
        let messages = assistant.init_messages();
        let layout = SessionLayout::new(&messages);
        SessionInner { assistant, layout }
    }

    async fn run(&mut self, messages: &[Message]) -> Result<(), Error> {
        enable_raw_mode()?;
        crossterm::execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
        let mut eventstream = crossterm::event::EventStream::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        if let Err(e) = self.layout.update(messages, None) {
            tracing::error!("error: {}", e);
        }

        term.draw(|f| {
            self.layout.draw(f);
        })?;
        loop {
            tokio::select! {
                // new input event
                input = eventstream.next() => {
                    if let Some(Ok(event)) = input {
                        tracing::trace!("event: {:?}", event);
                        match event.into() {
                            Input { key: Key::Esc, .. } => break,
                            Input {key: Key::Tab, ..} => {
                                self.layout.switch();
                            },
                            Input {
                                key: Key::Char('c'),
                                ..
                            } => break,
                            Input {
                                key: Key::Char('n'),
                                ctrl: true,
                                shift: _,
                                ..
                            } => {
                                self.layout.next_branch();
                                }
                            Input {
                                key: Key::Char('p'),
                                ctrl: true,
                                ..
                            } => {
                                self.layout.previous_branch();
                            }
                            Input {
                                key: Key::Char('u'),
                                ctrl: true,
                                ..
                            } => {
                                self.layout.up_one();
                            }
                            Input {
                                key: Key::Char('d'),
                                ctrl: true,
                                ..
                            } => {
                                self.layout.down_one();
                            }
                            Input {
                                key: Key::Char('i'),
                                ctrl: true,
                                ..
                            } => {
                                let messages = self.layout.messages();
                                tracing::debug!("messages: {:?}", messages);
                                self.assistant.handle_input(messages, tx.clone());
                            }
                            input => {
                                self.layout.input(input);
                            }
                        }
                    };
                    // Don't redraw if there are more events to process
                    if let Ok(true) = crossterm::event::poll(std::time::Duration::from_millis(0)) {
                        continue;
                    }
                    term.draw(|f| {
                        self.layout.draw(f);
                    })?;
                }
                tx = rx.recv() => {
                    if let Some(tx) = tx { self.layout.handle_assistant_event(tx).await }
                    term.draw(|f| {
                        self.layout.draw(f);
                    })?;
                }
            }
        }

        disable_raw_mode()?;
        crossterm::execute!(
            term.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        term.show_cursor()?;

        Ok(())
    }
}
