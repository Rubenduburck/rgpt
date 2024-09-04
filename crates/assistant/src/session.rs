use crate::textarea::SessionAreaId;
use crate::textarea::SessionTextArea;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::stream::StreamExt;
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
    pagetree::{NodeId, Root},
    Assistant,
};
use rgpt_types::{
    completion::TextEvent,
    message::{Message, Role},
};

pub struct Session {
    inner: SessionInner,
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

pub struct SessionLayout<'a> {
    pub page_tree: Root<'a>,
    pub current_node: NodeId,
    pub active: SessionAreaId,

    pub assistant_stream_node: Option<NodeId>,

    // FIXME: patch until tui-textarea implements wrapping.
    pub max_line_length: usize,
}

impl std::fmt::Debug for SessionLayout<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLayout")
            .field(
                "current_node",
                self.page_tree.get(self.current_node).unwrap(),
            )
            .field("active", &self.active)
            .field("max_line_length", &self.max_line_length)
            .finish()
    }
}

impl<'a> SessionLayout<'a> {
    fn new(messages: &[Message]) -> Self {
        tracing::trace!("messages: {:?}", messages);
        // FIXME: patch until tui-textarea implements wrapping.
        let max_line_length = crossterm::terminal::size()
            .map(|(w, _)| (w.saturating_sub(10)) as usize / 2)
            .unwrap_or(70);
        tracing::trace!("max_line_length: {}", max_line_length);

        let mut messages = messages.to_vec();
        messages.push(Message {
            role: Role::User,
            content: "".to_string(),
        });
        messages.push(Message {
            role: Role::Assistant,
            content: "".to_string(),
        });

        let mut page_tree = Root::new(max_line_length);
        let current_node = match page_tree.insert_messages(None, messages) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("error inserting messages: {}", e);
                NodeId::default()
            }
        };

        let active = SessionAreaId::User;
        let mut layout = SessionLayout {
            page_tree,
            current_node,
            active,
            max_line_length,
            assistant_stream_node: None,
        };
        layout.activate(active);
        layout.switch_node(current_node);
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
            SessionAreaId::System => self.page_tree.get_system_area(),
            _ => self.page_tree.get(self.current_node).unwrap().area(id),
        }
    }

    /// Get the parent node of the current node and return the area with the given id.
    /// Default to the node itself if the parent is not found (is root).
    fn parent_node_area(&self, area_id: SessionAreaId) -> &SessionTextArea<'a> {
        match area_id {
            SessionAreaId::System => self.page_tree.get_system_area(),
            _ => match self.page_tree.get(self.current_node).map(|n| n.parent) {
                Some(node @ NodeId::Node(_)) => self.page_tree.get(node).unwrap().area(area_id),
                _ => self.current_node_area(area_id),
            },
        }
    }

    fn current_node_area_mut(&mut self, id: SessionAreaId) -> &mut SessionTextArea<'a> {
        match id {
            SessionAreaId::System => self.page_tree.get_system_area_mut(),
            _ => self
                .page_tree
                .get_mut(self.current_node)
                .unwrap()
                .area_mut(id),
        }
    }

    fn activate(&mut self, id: SessionAreaId) {
        self.page_tree.activate(self.current_node, id);
        self.active = id;
    }

    fn switch_pane(&mut self) {
        self.activate(match self.active {
            SessionAreaId::User => SessionAreaId::Assistant,
            SessionAreaId::Assistant => SessionAreaId::System,
            SessionAreaId::System => SessionAreaId::User,
        });
    }

    fn input(&mut self, input: Input) {
        if !self.current_node_area_mut(self.active).input(input.clone()) {
            self.fork_current_node();
            self.current_node_area_mut(self.active).input(input);
        }
    }

    fn fork_current_node(&mut self) {
        let fork_id = self.page_tree.fork_node(self.current_node);
        self.switch_node(fork_id);
    }

    fn user_text_area_to_draw(&self) -> &TextArea {
        self.current_node_area(SessionAreaId::User).text_area()
    }

    fn assistant_text_area_to_draw(&self) -> &TextArea {
        match self.current_node_area(SessionAreaId::Assistant) {
            node if node.is_empty() => self.parent_node_area(SessionAreaId::Assistant).text_area(),
            node => node.text_area(),
        }
    }

    fn system_text_area_to_draw(&self) -> &TextArea {
        self.current_node_area(SessionAreaId::System).text_area()
    }

    fn draw(&mut self, f: &mut Frame) {
        tracing::debug!("layout: {:?}", self);
        let (outer_layout, user_layout) = self.chunks(f.area());
        let user_area = self.user_text_area_to_draw();
        let assistant_area = self.assistant_text_area_to_draw();
        let system_area = self.system_text_area_to_draw();
        f.render_widget(user_area, user_layout[1]);
        f.render_widget(assistant_area, outer_layout[1]);
        f.render_widget(system_area, user_layout[0]);
    }

    fn messages(&self) -> Vec<Message> {
        let mut messages = vec![Message::from(self.current_node_area(SessionAreaId::System))];
        messages.extend(self.page_tree.collect_messages(self.current_node, None));
        messages
    }

    fn switch_node(&mut self, node: NodeId) -> Option<NodeId> {
        self.current_node = node;
        self.activate(self.active);
        Some(node)
    }

    fn up_one(&mut self) -> Option<NodeId> {
        self.switch_node(self.page_tree.children(self.current_node).first()?.id)
    }

    fn down_one(&mut self) -> Option<NodeId> {
        self.switch_node(self.page_tree.parent(self.current_node)?.id)
    }

    fn next_branch(&mut self) -> Option<NodeId> {
        self.switch_node(self.page_tree.next_sibling(self.current_node)?.id)
    }

    fn previous_branch(&mut self) -> Option<NodeId> {
        self.switch_node(self.page_tree.previous_sibling(self.current_node)?.id)
    }

    fn new_branch(&mut self, node_id: NodeId) {
        let id = self.page_tree.insert_child_with_parent(
            self.page_tree
                .parent(node_id)
                .map_or(NodeId::Root, |n| n.id),
        );
        tracing::debug!("new branch {:?} from {:?}", id, node_id);
        self.switch_node(id);
    }

    fn new_branch_at_current(&mut self) {
        self.new_branch(self.current_node);
    }

    fn update(&mut self, messages: &[Message], node: Option<NodeId>) -> Result<(), Error> {
        let id = self.page_tree.insert_messages(node, messages.to_vec())?;
        self.switch_node(id);
        Ok(())
    }

    fn lock_current_node(&mut self) {
        self.page_tree.get_mut(self.current_node).unwrap().lock();
        self.assistant_stream_node = Some(self.current_node);
    }

    fn get_assistant_stream_node(&self) -> Option<NodeId> {
        self.assistant_stream_node
    }

    fn reset_assistant_stream_node(&mut self) {
        self.assistant_stream_node = None;
    }

    fn new_child(&mut self, node: NodeId) {
        let id = self.page_tree.insert_child_with_parent(node);
        self.switch_node(id);
    }

    fn new_child_at_current(&mut self) {
        self.new_child(self.current_node);
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
        tracing::trace!("assistant event: {:?}", event);
        let area = if let Some(node) = self.get_assistant_stream_node() {
            self.page_tree
                .get_mut(node)
                .unwrap()
                .area_mut(SessionAreaId::Assistant)
        } else {
            self.current_node_area_mut(SessionAreaId::Assistant)
        };
        match event {
            TextEvent::Null => {}
            TextEvent::MessageStart { .. } => {
                // clear the assistant buffer
                area.clear();
            }
            TextEvent::ContentBlockStart { content_block, .. } => {
                for input in string_to_inputs(content_block.text().unwrap_or_default().as_str()) {
                    area.force_input(input);
                }
            }
            TextEvent::ContentBlockDelta { delta, .. } => {
                for input in string_to_inputs(delta.text().unwrap_or_default().as_str()) {
                    area.force_input(input);
                }
            }
            TextEvent::ContentBlockStop { .. } => {}
            TextEvent::MessageDelta { .. } => {}
            TextEvent::MessageStop => {
                tracing::trace!("message stop");
                self.reset_assistant_stream_node();
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

        if !messages.is_empty() {
            if let Err(e) = self.layout.update(messages, None) {
                tracing::error!("error: {}", e);
            }
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
                                self.layout.switch_pane();
                            },
                            Input {
                                key: Key::Char('c'),
                                ctrl: true,
                                ..
                            } => break,
                            Input {
                                key: Key::Char('b'),
                                ctrl: true,
                                ..
                            } => {
                                self.layout.new_branch_at_current();
                            }
                            Input {
                                key: Key::Char('n'),
                                ctrl: true,
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
                                key: Key::Char('j'),
                                ctrl: true,
                                ..
                            } => {
                                let messages = self.layout.messages();
                                tracing::debug!("sending messages to assistant: {:?}", messages);
                                self.assistant.handle_input(messages, tx.clone());
                                self.layout.lock_current_node();
                                self.layout.new_child_at_current();
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
                    if let Some(event) = tx { self.layout.handle_assistant_event(event).await }
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
