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

#[derive(Default, Debug, Clone)]
struct CodeBlock {
    code: Vec<u8>,
    lang: Option<String>,
}

impl CodeBlock {
    fn new(lang: Option<String>) -> Self {
        Self { code: vec![], lang }
    }
    fn push(&mut self, msg: Vec<u8>) {
        self.code.extend(msg.iter().copied());
    }

    fn bytes(&self) -> Vec<u8> {
        self.code.clone()
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes()).to_string()
    }

    fn split(&self) -> Vec<CodeBlock> {
        self.bytes()
            .split(|&b| b == b'\n')
            .map(|part| CodeBlock {
                code: part.to_vec(),
                lang: self.lang.clone(),
            })
            .collect()
    }
}

impl QueryState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_message(&mut self, index: usize, msg: Vec<u8>) {
        if self.messages.len() <= index {
            self.messages.resize(index + 1, vec![]);
        }
        self.messages
            .get_mut(index)
            .unwrap()
            .extend(msg.iter().copied());
    }

    fn get_code_blocks(&self) -> Vec<CodeBlock> {
        let mut code_blocks: Vec<CodeBlock> = vec![];
        let mut inside_code_block = false;
        let mut current_block: Option<CodeBlock> = None;

        for msg in self.messages.iter() {
            let parts: Vec<&[u8]> = msg.split(|&b| b == b'\n').collect();

            for part in parts {
                match (inside_code_block, part.starts_with(b"```")) {
                    (true, true) => {
                        inside_code_block = false;
                        if let Some(block) = current_block.take() {
                            code_blocks.push(block);
                        }
                    }
                    (true, false) => {
                        if let Some(ref mut block) = current_block {
                            block.code.extend_from_slice(part);
                            block.code.push(b'\n');
                        }
                    }
                    (false, true) => {
                        let lang = String::from_utf8_lossy(&part[3..])
                            .split_whitespace()
                            .next()
                            .map(|s| s.to_string());
                        current_block = Some(CodeBlock::new(lang));
                        inside_code_block = true;
                    }
                    (false, false) => {}
                }
            }
        }

        // Handle case where the last code block isn't closed
        if let Some(block) = current_block {
            code_blocks.push(block);
        }

        code_blocks
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

        let (resp_tx, mut resp_rx) = tokio::sync::mpsc::channel(10);
        self.assistant.handle_input(messages, resp_tx);

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
            std::io::stdout().write_all(b"\n")?;
            match Self::select(&self.state.get_code_blocks()) {
                None => {}
                Some(code_block) => {
                    let code = code_block.bytes();
                    let mut cmd = Command::new("bash");
                    cmd.stdin(std::process::Stdio::piped());
                    let mut child = cmd.spawn()?;
                    child.stdin.as_mut().unwrap().write_all(&code)?;
                    let output = child.wait_with_output()?;
                    std::io::stdout().write_all(&output.stdout)?;
                }
            }
        }

        Ok(())
    }

    fn select(code_blocks: &[CodeBlock]) -> Option<CodeBlock> {
        const ALLOWED_LANGS: [&str; 3] = ["bash", "zsh", "sh"];
        let code_blocks = code_blocks
            .iter()
            .filter(|block| {
                ALLOWED_LANGS.contains(&block.lang.as_deref().unwrap_or("")) || block.lang.is_none()
            })
            .flat_map(|block| block.split())
            .filter(|block| !block.text().is_empty())
            .collect::<Vec<CodeBlock>>();

        if code_blocks.is_empty() {
            return None;
        }

        let selections = std::iter::once(String::from("None"))
            .chain(code_blocks.iter().map(|block| block.text()))
            .collect::<Vec<String>>();

        match dialoguer::Select::new()
            .with_prompt("Execute?")
            .items(&selections)
            .default(0)
            .interact()
        {
            Ok(0) => None,
            Ok(selection) => Some(code_blocks[selection - 1].clone()),
            Err(e) => {
                tracing::error!("error: {}", e);
                None
            }
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
            TextEvent::MessageDelta { delta } => Ok(vec![]),
            TextEvent::MessageStop => Ok(vec![]),

            TextEvent::ContentBlockStart {
                index,
                content_block,
            } => self.handle_content_block_start(index, content_block),
            TextEvent::ContentBlockDelta { index, delta } => {
                self.handle_content_block_delta(index, delta)
            }
            TextEvent::ContentBlockStop { index } => Ok(vec![]),
            _ => Ok(vec![]),
        }
    }

    pub fn handle_message_bytes(&mut self, index: usize, msg: Vec<u8>) -> Result<Vec<u8>, Error> {
        self.state.add_message(index, msg.clone());
        let diff = index - self.state.line_no;
        self.state.line_no = index;
        Ok(std::iter::repeat(b'\n').take(diff).chain(msg).collect())
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
        let state = QueryState {
            messages: vec![
                b"```rust".to_vec(),
                b"fn main() {".to_vec(),
                b"    println!(\"Hello, World!\");".to_vec(),
                b"}".to_vec(),
                b"```".to_vec(),
                b"```bash".to_vec(),
                b"ls".to_vec(),
                b"```".to_vec(),
                b"```bash\nls\n```".to_vec(),
            ],
            ..Default::default()
        };

        let code_blocks = state.get_code_blocks();
        println!("{:?}", code_blocks);
        assert_eq!(code_blocks.len(), 3);
    }
}
