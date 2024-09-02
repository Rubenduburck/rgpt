use rgpt_types::{
    completion::{ContentBlock, ContentDelta, MessageStartData, TextEvent},
    message::{Message, Role},
};

pub enum StateRequest {
    MessageEvent(Vec<Message>),
    PushAssistantEvent(TextEvent),
    GetPromptMessages(tokio::sync::oneshot::Sender<Vec<Message>>),
}

pub struct StateInner {
    user_buffers: Vec<Vec<ContentBlock>>,
    assistant_buffers: Vec<Vec<ContentBlock>>,
    system_buffers: Vec<Vec<ContentBlock>>,

    start_messages: Vec<MessageStartData>,

    last_drawn_lines: u16,
}

impl StateInner {
    pub fn new() -> Self {
        StateInner {
            user_buffers: vec![vec![]],
            assistant_buffers: vec![],
            system_buffers: vec![],
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

    pub async fn handle_messages(&mut self, messages: Vec<Message>) {
        fn to_content_blocks(content: &str) -> Vec<ContentBlock> {
            content
                .lines()
                .map(|line| ContentBlock::Text {
                    text: line.to_string(),
                })
                .collect()
        }
        for message in messages {
            match message.role {
                Role::User => {
                    self.user_buffers.push(to_content_blocks(&message.content));
                }
                Role::Assistant => {
                    self.assistant_buffers
                        .push(to_content_blocks(&message.content));
                }
                Role::System => {
                    self.system_buffers
                        .push(to_content_blocks(&message.content));
                }
            }
        }
    }

    pub async fn handle_requests(
        &mut self,
        rx: &mut tokio::sync::mpsc::Receiver<StateRequest>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(request) = rx.recv().await {
            match request {
                StateRequest::MessageEvent(messages) => {
                    self.handle_messages(messages).await;
                }
                StateRequest::PushAssistantEvent(event) => self.push_assistant_event(event),
                StateRequest::GetPromptMessages(tx) => {
                    let _ = tx.send(self.get_user_message());
                }
            }
        }
        Ok(())
    }

    pub fn push_user_event(&mut self, event: String) {
        let content_blocks = event
            .lines()
            .map(|line| ContentBlock::Text {
                text: line.to_string(),
            })
            .collect::<Vec<_>>();
        self.user_buffers.push(content_blocks);
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

    // Get full history of messages
    // First message is a system message if it exists
    // After that alternating user and assistant messages
    pub fn get_user_message(&self) -> Vec<Message> {
        fn to_messages(buffers: &[Vec<ContentBlock>], role: Role) -> Vec<Message> {
            buffers
                .iter()
                .map(|buffer| {
                    let content = buffer
                        .iter()
                        .map(|block| block.text().unwrap_or_default().to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    Message { role: role.clone(), content }
                })
                .collect()
        }
        let system_messages = to_messages(&self.system_buffers, Role::System);
        let user_messages = to_messages(&self.user_buffers, Role::User);
        let assistant_messages = to_messages(&self.assistant_buffers, Role::Assistant);

        let mut messages = system_messages;
        for (user, assistant) in user_messages.into_iter().zip(assistant_messages) {
            messages.push(user);
            messages.push(assistant);
        }
        messages
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

#[derive(Clone)]
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

    pub async fn push_messages(&self, messages: &[Message]) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.tx.send(StateRequest::MessageEvent(messages.to_vec())).await?)
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

    pub async fn get_prompt_messages(&self) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(StateRequest::GetPromptMessages(tx)).await?;
        Ok(rx.await?)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
