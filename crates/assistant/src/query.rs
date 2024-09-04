use std::{io::Write as _, process::Command};

use crate::{error::Error, Assistant};
use rgpt_types::{
    completion::{Content, ContentBlock, ContentDelta, TextEvent},
    message::Message,
};

pub struct Query {
    assistant: Assistant,
    state: QueryState,
    execute: bool,
}

#[derive(Default)]
pub struct QueryState {
    line_no: usize,
    messages: Vec<Vec<u8>>,
}

type CodeBlock = Vec<u8>;

impl QueryState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_message(&mut self, index: usize, msg: Vec<u8>) {
        if self.messages.len() <= index {
            self.messages.resize(index + 1, vec![]);
        }
        self.line_no += msg.iter().filter(|&&b| b == b'\n').count();
        self.messages
            .get_mut(index)
            .unwrap()
            .extend(msg.iter().copied());
    }

    fn get_code_blocks(&self) -> Vec<Vec<u8>> {
        let joined = self.messages.iter().flatten().copied().collect::<Vec<u8>>();
        let mut blocks = Vec::new();
        let mut current_block = Vec::new();

        for line in joined.split(|&b| b == b'\n') {
            if !line.is_empty() {
                current_block.extend_from_slice(line);
                current_block.push(b'\n');

                if !line.ends_with(b"/") {
                    blocks.push(current_block);
                    current_block = Vec::new();
                }
            }
        }

        // Add the last block if it's not empty
        if !current_block.is_empty() {
            blocks.push(current_block);
        }

        blocks
    }
}

impl Query {
    const ANSI_BLUE_START: &'static [u8] = b"\x1b[94m";
    const ANSI_BLUE_END: &'static [u8] = b"\x1b[0m";
    const ANSI_PURPLE_START: &'static [u8] = b"\x1b[95m";
    const ANSI_PURPLE_END: &'static [u8] = b"\x1b[0m";

    fn assistant_write(msg: Vec<u8>) -> Result<(), Error> {
        std::io::stdout().write_all(Self::ANSI_PURPLE_START)?;
        std::io::stdout().write_all(&msg)?;
        std::io::stdout().write_all(Self::ANSI_PURPLE_END)?;
        std::io::stdout().flush()?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn start(&mut self, messages: &[Message]) -> Result<(), Error> {
        tracing::debug!("messages: {:?}", messages);
        tracing::debug!("assistant: {:?}", self.assistant);
        let messages = if messages.is_empty() {
            Self::prompt_user_input().await?
        } else {
            messages.to_vec()
        };
        let mut query_messages = self.assistant.init_messages();
        query_messages.extend(messages);

        let (resp_tx, mut resp_rx) = tokio::sync::mpsc::channel(10);
        self.assistant.handle_input(query_messages, resp_tx);

        let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(10);
        tokio::spawn(async move {
            tracing::debug!("output task started");
            while let Some(msg) = out_rx.recv().await {
                Self::assistant_write(msg)?;
            }
            Ok::<(), Error>(())
        });

        while let Some(event) = resp_rx.recv().await {
            tracing::debug!("event: {:?}", event);
            let _ = out_tx.send(self.handle_event(event)?).await;
        }

        if self.execute {
            // Clear the current line instead of adding a newline
            print!("\r\x1b[K");
            std::io::stdout().flush()?;

            match self.select(&self.state.get_code_blocks()) {
                None => {}
                Some(code) => {
                    let mut cmd = Command::new("bash");
                    cmd.stdin(std::process::Stdio::piped());
                    cmd.stdout(std::process::Stdio::piped());
                    cmd.stderr(std::process::Stdio::piped());
                    let mut child = cmd.spawn()?;
                    child.stdin.as_mut().unwrap().write_all(&code)?;
                    let output = child.wait_with_output()?;

                    // Print both stdout and stderr
                    std::io::stdout().write_all(&output.stdout)?;
                    std::io::stderr().write_all(&output.stderr)?;

                    // Ensure everything is flushed
                    std::io::stdout().flush()?;
                    std::io::stderr().flush()?;

                    if !output.stdout.ends_with(b"\n") && !output.stderr.ends_with(b"\n") {
                        println!();
                    }
                }
            }
        }
        Ok(())
    }

    fn select(&self, code_blocks: &[CodeBlock]) -> Option<CodeBlock> {
        // Jump back up self.state.line_no lines
        for _ in 0..self.state.line_no {
            let _ = std::io::stdout().write_all(b"\x1b[A");
        }
        std::io::stdout().flush().unwrap();

        let exit = [Query::ANSI_BLUE_START, b"exit ", Query::ANSI_BLUE_END].concat();
        if code_blocks.is_empty() {
            return None;
        }

        let selections = code_blocks
            .iter()
            .map(|block| {
                format!(
                    "{}{}{}",
                    String::from_utf8_lossy(Self::ANSI_PURPLE_START),
                    String::from_utf8_lossy(block).trim(),
                    String::from_utf8_lossy(Self::ANSI_PURPLE_END),
                )
            })
            .chain(std::iter::once(String::from_utf8_lossy(&exit).to_string()))
            .collect::<Vec<String>>();

        match dialoguer::Select::new()
            .items(&selections)
            .default(selections.len() - 1)
            .interact()
        {
            Ok(selection) if selection == selections.len() - 1 => None,
            Ok(selection) => Some(code_blocks[selection].clone()),
            Err(_e) => None,
        }
    }

    #[tracing::instrument]
    pub async fn prompt_user_input() -> Result<Vec<Message>, Error> {
        std::io::stdout().write_all(b"> ")?;
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(vec![Message::from(line.trim().to_string())])
    }

    #[tracing::instrument(skip(self))]
    pub fn handle_event(&mut self, event: TextEvent) -> Result<Vec<u8>, Error> {
        match event {
            TextEvent::MessageStart { message } => message
                .content
                .into_iter()
                .enumerate()
                .try_fold(vec![], |mut acc, (i, content)| {
                    acc.extend(self.handle_content(i, content)?);
                    Ok(acc)
                }),
            TextEvent::MessageDelta { .. } => Ok(vec![]),
            TextEvent::MessageStop => Ok(vec![]),

            TextEvent::ContentBlockStart {
                index,
                content_block,
            } => self.handle_content_block_start(index, content_block),
            TextEvent::ContentBlockDelta { index, delta } => {
                self.handle_content_block_delta(index, delta)
            }
            TextEvent::ContentBlockStop { .. } => Ok(vec![]),
            _ => Ok(vec![]),
        }
    }

    pub fn handle_message_bytes(&mut self, index: usize, msg: Vec<u8>) -> Result<Vec<u8>, Error> {
        self.state.add_message(index, msg.clone());
        Ok(msg)
    }

    pub fn handle_content(&mut self, index: usize, content: Content) -> Result<Vec<u8>, Error> {
        self.handle_message_bytes(index, content.bytes())
    }

    pub fn handle_content_block_start(
        &mut self,
        index: usize,
        block: ContentBlock,
    ) -> Result<Vec<u8>, Error> {
        self.handle_message_bytes(index, block.bytes())
    }

    pub fn handle_content_block_delta(
        &mut self,
        index: usize,
        delta: ContentDelta,
    ) -> Result<Vec<u8>, Error> {
        self.handle_message_bytes(index, delta.bytes())
    }

    pub fn builder(assistant: Assistant) -> Builder {
        Builder::new(assistant)
    }
}

pub struct Builder {
    assistant: Assistant,
    execute: bool,
}

impl Builder {
    pub fn new(assistant: Assistant) -> Self {
        Self {
            execute: false,
            assistant,
        }
    }

    pub fn execute(mut self, execute: bool) -> Self {
        self.execute = execute;
        self
    }

    pub fn build(self) -> Query {
        Query {
            execute: self.execute,
            assistant: self.assistant,
            state: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_code_block() {
        let mut state = QueryState::new();
        state.add_message(0, b"echo 'Hello, World!'\n".to_vec());
        state.add_message(1, b"echo 'Goodbye, World!'\n".to_vec());
        state.add_message(2, b"echo 'Hello, World!'\n".to_vec());
        state.add_message(3, b"echo 'Goodbye, World!'\n".to_vec());
        state.add_message(
            4,
            b"echo 'Hello, World!'/\necho 'Goodbye, World!'\n".to_vec(),
        );

        let blocks = state.get_code_blocks();
        assert_eq!(blocks.len(), 5);
    }
}
