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

use crate::{error::Error, Assistant};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAreaId {
    User,
    Assistant,
    System,
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

pub struct SessionTextArea<'a> {
    pub id: SessionAreaId,
    pub page: usize,
    pub text_area: TextArea<'a>,

    // FIXME: patch until tui-textarea implements wrapping.
    pub max_line_length: usize,
}

impl<'a> SessionTextArea<'a> {
    fn new(id: SessionAreaId, lines: &[&str], page: usize, max_line_length: usize) -> Self {
        let mut text_area = TextArea::from(lines.iter().map(|l| l.to_string()).collect::<Vec<_>>());
        text_area.set_cursor_line_style(Style::default());
        let mut session_text_area = SessionTextArea {
            id,
            page,
            text_area,
            max_line_length,
        };
        session_text_area.inactivate();
        session_text_area
    }

    fn title(&self) -> String {
        format!("{}: {}", String::from(self.id), self.page)
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

    fn activate(&mut self) {
        self.text_area
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        self.text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default())
                .title(self.title()),
        );
    }

    fn inactivate(&mut self) {
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
    pub system_area: SessionTextArea<'a>,
    pub user_areas: Vec<SessionTextArea<'a>>,
    pub assistant_areas: Vec<SessionTextArea<'a>>,
    pub page: usize,
    pub active: SessionAreaId,

    // FIXME: patch until tui-textarea implements wrapping.
    pub max_line_length: usize,
}

impl<'a> SessionLayout<'a> {
    fn debug_print(&self) {
        tracing::trace!("page: {}", self.page);
        tracing::trace!("active: {:?}", self.active);
        tracing::trace!("user_areas: {:?}", self.user_areas.len());
        tracing::trace!("assistant_areas: {:?}", self.assistant_areas.len());
    }
    fn new(messages: &[Message]) -> Self {
        tracing::trace!("messages: {:?}", messages);
        // FIXME: patch until tui-textarea implements wrapping.
        let max_line_length = crossterm::terminal::size()
            .map(|(w, _)| (w.saturating_sub(10)) as usize / 2)
            .unwrap_or(70);
        tracing::trace!("max_line_length: {}", max_line_length);
        let active = SessionAreaId::User;
        let mut user_areas = messages
            .iter()
            .filter(|m| m.role == Role::User)
            .enumerate()
            .map(|(page, m)| {
                SessionTextArea::new(
                    SessionAreaId::User,
                    m.content.lines().collect::<Vec<_>>().as_slice(),
                    page,
                    max_line_length,
                )
            })
            .collect::<Vec<_>>();
        user_areas.push(SessionTextArea::new(
            SessionAreaId::User,
            &[],
            user_areas.len(),
            max_line_length,
        ));
        let mut assistant_areas = messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .enumerate()
            .map(|(page, m)| {
                SessionTextArea::new(
                    SessionAreaId::Assistant,
                    m.content.lines().collect::<Vec<_>>().as_slice(),
                    page,
                    max_line_length,
                )
            })
            .collect::<Vec<_>>();
        assistant_areas.push(SessionTextArea::new(
            SessionAreaId::Assistant,
            &[],
            assistant_areas.len(),
            max_line_length,
        ));
        let system_message_lines = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.lines().collect::<Vec<_>>().as_slice().to_vec());
        let system_area = SessionTextArea::new(
            SessionAreaId::System,
            system_message_lines.as_deref().unwrap_or_default(),
            0,
            max_line_length,
        );
        let page = user_areas.len() - 1;
        let mut layout = SessionLayout {
            system_area,
            user_areas,
            assistant_areas,
            page,
            active: SessionAreaId::User,
            max_line_length,
        };
        layout.area_mut(active).unwrap().activate();
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

    fn area_mut(&mut self, id: SessionAreaId) -> Option<&mut SessionTextArea<'a>> {
        self.page_area_mut(self.page, id)
    }

    fn area(&self, id: SessionAreaId) -> Option<&SessionTextArea<'a>> {
        self.page_area(self.page, id)
    }

    fn page_area(&self, page: usize, id: SessionAreaId) -> Option<&SessionTextArea<'a>> {
        match id {
            SessionAreaId::User => self.user_areas.get(page),
            SessionAreaId::Assistant => self.assistant_areas.get(page),
            SessionAreaId::System => Some(&self.system_area),
        }
    }

    fn page_area_mut(
        &mut self,
        page: usize,
        id: SessionAreaId,
    ) -> Option<&mut SessionTextArea<'a>> {
        match id {
            SessionAreaId::User => self.user_areas.get_mut(page),
            SessionAreaId::Assistant => self.assistant_areas.get_mut(page),
            SessionAreaId::System => Some(&mut self.system_area),
        }
    }

    fn active_area(&self) -> &SessionTextArea<'a> {
        self.area(self.active).unwrap()
    }

    fn active_area_mut(&mut self) -> &mut SessionTextArea<'a> {
        self.area_mut(self.active).unwrap()
    }

    fn switch(&mut self) {
        self.active = match self.active {
            SessionAreaId::User => SessionAreaId::Assistant,
            SessionAreaId::Assistant => SessionAreaId::System,
            SessionAreaId::System => SessionAreaId::User,
        };
        for id in [
            SessionAreaId::User,
            SessionAreaId::Assistant,
            SessionAreaId::System,
        ]
        .iter()
        {
            if *id == self.active {
                if let Some(a) = self.area_mut(*id) {
                    a.activate()
                }
            } else if let Some(a) = self.area_mut(*id) {
                a.inactivate()
            }
        }
    }

    fn input(&mut self, input: Input) {
        self.active_area_mut().input(input);
    }

    fn draw(&self, f: &mut Frame) {
        let (outer_layout, user_layout) = self.chunks(f.area());
        let user_area = self.area(SessionAreaId::User).unwrap();
        let assistant_area = match self.area(SessionAreaId::Assistant) {
            Some(assistant_area) if assistant_area.is_empty() => self
                .page_area(self.page.saturating_sub(1), SessionAreaId::Assistant)
                .unwrap(),
            Some(assistant_area) => assistant_area,
            None => {
                panic!("assistant area is empty");
            }
        };
        let system_area = self.area(SessionAreaId::System).unwrap();
        f.render_widget(user_area.text_area(), user_layout[1]);
        f.render_widget(assistant_area.text_area(), outer_layout[1]);
        f.render_widget(system_area.text_area(), user_layout[0]);
    }

    fn messages(&self) -> Vec<Message> {
        let mut messages = vec![Message::from(self.area(SessionAreaId::System).unwrap())];
        for i in 0..self.page {
            messages.push(Message::from(&self.user_areas[i]));
            messages.push(Message::from(&self.assistant_areas[i]));
        }
        messages.push(Message::from(&self.user_areas[self.page]));
        messages
    }

    fn next_page(&mut self) {
        self.page = (self.page + 1) % self.user_areas.len();
        self.activate(self.active);
    }

    fn previous_page(&mut self) {
        self.page = (self.page + self.user_areas.len() - 1) % self.user_areas.len();
        self.activate(self.active);
    }

    fn inactivate_all(&mut self) {
        for id in [
            SessionAreaId::User,
            SessionAreaId::Assistant,
            SessionAreaId::System,
        ]
        .iter()
        {
            if let Some(a) = self.area_mut(*id) {
                a.inactivate()
            }
        }
    }

    fn activate(&mut self, id: SessionAreaId) {
        self.inactivate_all();
        if let Some(a) = self.area_mut(id) {
            a.activate()
        }
    }

    fn new_page(&mut self) {
        self.inactivate_all();
        if self.user_areas.last().unwrap().lines().is_empty()
            && self.assistant_areas.last().unwrap().lines().is_empty()
        {
            return;
        }
        self.user_areas.push(SessionTextArea::new(
            SessionAreaId::User,
            &[],
            self.page + 1,
            self.max_line_length,
        ));
        self.assistant_areas.push(SessionTextArea::new(
            SessionAreaId::Assistant,
            &[],
            self.page + 1,
            self.max_line_length,
        ));
        self.next_page();
    }

    fn update(&mut self, page: usize, messages: &[Message]) {
        for message in messages {
            match message.role {
                Role::User => {
                    self.user_areas[page].text_area = TextArea::from(message.content.lines());
                }
                Role::Assistant => {
                    self.assistant_areas[page].text_area = TextArea::from(message.content.lines());
                }
                Role::System => {
                    self.system_area.text_area = TextArea::from(message.content.lines());
                }
            }
        }
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
        let area = &mut self.assistant_areas[self.page];
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
                self.new_page();
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

        term.draw(|f| {
            self.layout.draw(f);
        })?;
        loop {
            self.layout.debug_print();
            tokio::select! {
                // new input event
                input = eventstream.next() => {
                    if let Some(Ok(event)) = input {
                        match event.into() {
                            Input { key: Key::Esc, .. } => break,
                            Input {key: Key::Tab, ..} => {
                                self.layout.switch();
                            },
                            Input {
                                key: Key::Char('c'),
                                ctrl: true,
                                ..
                            } => break,
                            Input {
                                key: Key::Char('n'),
                                ctrl: true,
                                shift,
                                ..
                            } => {
                                if shift {
                                    self.layout.new_page();
                                } else {
                                    self.layout.next_page();
                                }
                            }
                            Input {
                                key: Key::Char('p'),
                                ctrl: true,
                                ..
                            } => {
                                self.layout.previous_page();
                            }
                            Input {
                                key: Key::Enter, ..
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

    //fn draw_ui<B: Backend>(f: &mut Frame, input_buffer: &str, output_lines: &[String]) {
    //    let chunks = Layout::default()
    //        .direction(Direction::Horizontal)
    //        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
    //        .split(f.area());
    //
    //    let input_block = Block::default().title("Input").borders(Borders::ALL);
    //    let input = Paragraph::new(input_buffer)
    //        .style(Style::default().fg(Color::Yellow))
    //        .block(input_block);
    //    f.render_widget(input, chunks[0]);
    //
    //    let output_block = Block::default().title("Output").borders(Borders::ALL);
    //    let output_text: Vec<Span> = output_lines
    //        .iter()
    //        .map(|line| Span::raw(line.clone()))
    //        .collect();
    //    let output = Paragraph::new(output_text).block(output_block);
    //    f.render_widget(output, chunks[1]);
    //}
}
