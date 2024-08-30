use std::io::{stdout, Write as _};

use crossterm::{cursor, execute, style, terminal};
use rgpt_types::{
    completion::{ContentBlock, ContentDelta, MessageStartData, TextEvent},
    message::Message,
};

pub enum StateRequest {
    PushUserEvent(TextEvent),
    PushAssistantEvent(TextEvent),
    GetUserMessage(tokio::sync::oneshot::Sender<Vec<Message>>),
}

pub struct StateInner {
    user_buffers: Vec<Vec<ContentBlock>>,
    assistant_buffers: Vec<Vec<ContentBlock>>,
    start_messages: Vec<MessageStartData>,

    last_drawn_lines: u16,
}

impl StateInner {
    pub fn new() -> Self {
        StateInner {
            user_buffers: vec![vec![]],
            assistant_buffers: vec![],
            start_messages: vec![],

            last_drawn_lines: 0,
        }
    }

    pub async fn run(
        &mut self,
        mut rx: tokio::sync::mpsc::Receiver<StateRequest>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.handle_requests(&mut rx).await
    }

    // A function that draws the current state of the assistant
    pub fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let print = |s: &str, line: u16| {
            println!("{}/{}| {}", line, self.last_drawn_lines, s);
            stdout().flush().unwrap();
        };
        // Clear the screen

        if self.last_drawn_lines > 0 {
            execute!(stdout(), cursor::MoveUp(self.last_drawn_lines))?;
        }

        let mut current_line = 0;

        // Determine the maximum number of buffers
        let max_buffers = self.user_buffers.len().max(self.assistant_buffers.len());

        // Draw buffers alternating between user and assistant
        for i in 0..max_buffers {
            // Draw user buffer if available
            if i < self.user_buffers.len() {
                execute!(stdout(), style::SetForegroundColor(style::Color::Blue))?;
                for block in &self.user_buffers[i] {
                    print(&block.text().unwrap_or_default(), current_line);
                    current_line += 1;
                }
            }

            // Draw assistant buffer if available
            if i < self.assistant_buffers.len() {
                execute!(stdout(), style::SetForegroundColor(style::Color::Green))?;
                for block in &self.assistant_buffers[i] {
                    print(&block.text().unwrap_or_default(), current_line);
                    current_line += 1;
                }
            }
        }

        // Clear any remaining lines
        for _ in current_line..self.last_drawn_lines {
            execute!(stdout(), terminal::Clear(terminal::ClearType::CurrentLine))?;
            execute!(stdout(), cursor::MoveToNextLine(1))?;
        }

        // Reset color and flush stdout
        execute!(stdout(), style::SetForegroundColor(style::Color::Reset))?;
        stdout().flush()?;

        self.last_drawn_lines = current_line;

        Ok(())
    }

    pub async fn handle_requests(
        &mut self,
        rx: &mut tokio::sync::mpsc::Receiver<StateRequest>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.draw()?;
        while let Some(request) = rx.recv().await {
            match request {
                StateRequest::PushUserEvent(event) => self.push_user_event(event),
                StateRequest::PushAssistantEvent(event) => self.push_assistant_event(event),
                StateRequest::GetUserMessage(tx) => {
                    let _ = tx.send(self.get_user_message());
                }
            }
            self.draw()?;
        }
        Ok(())
    }

    pub fn push_user_event(&mut self, event: TextEvent) {
        pub fn new_content_block(
            state: &mut StateInner,
            _index: usize,
            content_block: ContentBlock,
        ) {
            if state.user_buffers.last().unwrap().is_empty() {
                state.user_buffers.last_mut().unwrap().push(content_block);
            } else {
                state.user_buffers.push(vec![content_block]);
            }
        }

        pub fn update_content_block(state: &mut StateInner, index: usize, delta: ContentDelta) {
            let buffer = state.user_buffers.last_mut().unwrap();
            if let Some(block) = buffer.get_mut(index) {
                block.update(&delta);
            }
        }

        match event {
            TextEvent::MessageStart { .. } => {}
            TextEvent::ContentBlockStart {
                index,
                content_block,
            } => new_content_block(self, index, content_block),
            TextEvent::ContentBlockDelta { index, delta } => {
                update_content_block(self, index, delta)
            }
            TextEvent::MessageStop => {}
            TextEvent::Null => {}
            TextEvent::MessageDelta { .. } => {}
            TextEvent::ContentBlockStop { .. } => {}
        }
    }

    pub fn push_assistant_event(&mut self, event: TextEvent) {
        pub fn push_start_message(state: &mut StateInner, message: MessageStartData) {
            state.start_messages.push(message);
        }

        pub fn new_content_block(
            state: &mut StateInner,
            _index: usize,
            content_block: ContentBlock,
        ) {
            state.assistant_buffers.push(vec![content_block]);
        }

        pub fn update_content_block(state: &mut StateInner, index: usize, delta: ContentDelta) {
            let buffer = state.assistant_buffers.last_mut().unwrap();
            if let Some(block) = buffer.get_mut(index) {
                block.update(&delta);
            }
        }
        match event {
            TextEvent::MessageStart { message } => push_start_message(self, message),
            TextEvent::ContentBlockStart {
                index,
                content_block,
            } => new_content_block(self, index, content_block),
            TextEvent::ContentBlockDelta { index, delta } => {
                update_content_block(self, index, delta)
            }
            TextEvent::MessageStop => {}
            TextEvent::Null => {}
            TextEvent::MessageDelta { .. } => {}
            TextEvent::ContentBlockStop { .. } => {}
        }
    }

    pub fn get_user_message(&self) -> Vec<Message> {
        self.user_buffers
            .last()
            .map(|buffer| {
                let role = "user".to_string();
                let content = buffer
                    .iter()
                    .map(|block| block.text().unwrap_or_default().to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                vec![Message { role, content }]
            })
            .unwrap_or_default()
    }

    pub fn get_assistant_buffer(&self) -> Vec<ContentBlock> {
        self.assistant_buffers
            .last()
            .map(|buffer| buffer.to_vec())
            .unwrap_or_default()
    }
}

impl Default for StateInner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct State {
    tx: tokio::sync::mpsc::Sender<StateRequest>,
}

impl State {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            let mut state = StateInner::new();
            if let Err(e) = state.run(rx).await {
                tracing::error!("Error in state: {:?}", e);
            }
        });
        State { tx }
    }

    pub async fn push_user_event(
        &self,
        event: TextEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.tx.send(StateRequest::PushUserEvent(event)).await?)
    }

    pub async fn push_assistant_event(
        &self,
        event: TextEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self
            .tx
            .send(StateRequest::PushAssistantEvent(event))
            .await?)
    }

    pub async fn get_user_messages(&self) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(StateRequest::GetUserMessage(tx)).await?;
        Ok(rx.await?)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
