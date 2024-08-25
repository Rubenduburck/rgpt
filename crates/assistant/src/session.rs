use rgpt_types::completion::Message;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

use crate::{error::Error, Assistant};

pub type UserInputTx = tokio::sync::mpsc::Sender<String>;
pub type UserInputRx = tokio::sync::mpsc::Receiver<String>;
pub type SystemOutputRx = tokio::sync::mpsc::Receiver<String>;
pub type SystemOutputTx = tokio::sync::mpsc::Sender<String>;
pub type UserKillTx = tokio::sync::mpsc::Sender<()>;
pub type UserKillRx = tokio::sync::mpsc::Receiver<()>;

pub struct Session {
    handles: Vec<tokio::task::JoinHandle<()>>,
    kill_tx: UserKillTx,
}

impl Session {
    pub async fn start(assistant: Assistant, messages: &[Message]) -> Result<(), Error> {
        async fn handle_user_input(input_tx: UserInputTx) -> Result<(), Error> {
            let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap() == 0 {
                    return Ok(());
                }
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                input_tx.send(line).await.unwrap();
            }
        }

        async fn handle_output(mut output_rx: SystemOutputRx) -> Result<(), Error> {
            let mut stdout = tokio::io::stdout();
            while let Some(output) = output_rx.recv().await {
                stdout.write_all(output.as_bytes()).await?;
                stdout.flush().await?;
            }
            Ok(())
        }
        let (mut session, input_tx, output_rx, kill_tx) = SessionInner::new(assistant);

        let session_handle = tokio::spawn(async move {
            if let Err(e) = session.run().await {
                tracing::error!("error: {}", e);
            }
        });

        for message in messages {
            if let Err(e) = input_tx.send(message.content.clone()).await {
                tracing::error!("error: {}", e);
                return Err(Error::SendInput);
            }
        }

        let input_handle = tokio::spawn(async move {
            if let Err(e) = handle_user_input(input_tx).await {
                tracing::error!("error: {}", e);
            }
        });

        handle_output(output_rx).await?;

        let _ = tokio::try_join!(session_handle, input_handle);
        Ok(())

    }
}

pub struct SessionInner {
    input_rx: UserInputRx,
    output_tx: SystemOutputTx,
    kill_rx: UserKillRx,
    assistant: Assistant,
    buffer: Vec<Message>,
}

impl SessionInner {
    fn new(assistant: Assistant) -> (Self, UserInputTx, SystemOutputRx, UserKillTx) {
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(100);
        let (output_tx, output_rx) = tokio::sync::mpsc::channel(100);
        let (kill_tx, kill_rx) = tokio::sync::mpsc::channel(100);

        let session = SessionInner {
            input_rx,
            output_tx,
            kill_rx,
            assistant,
            buffer: vec![],
        };

        (session, input_tx, output_rx, kill_tx)
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(input) = self.input_rx.recv().await {
            if input == "exit" {
                break;
            }
            self.buffer.push(input.into());

            let (tx, mut rx) = tokio::sync::mpsc::channel(100);
            self.assistant.handle_input(self.buffer.clone(), tx);
            self.buffer.clear();
            loop {
                tokio::select! {
                    _ = self.kill_rx.recv() => break,
                    event = rx.recv() => {
                        if let Some(event) = event {
                            if let Some(text) = event.text() {
                                self.output_tx.send(text.clone()).await?;
                            }
                            if event.is_stop() {
                                self.output_tx.send("\n".to_string()).await?;
                            }
                            if event.is_complete() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }

            self.output_tx.send("> ".to_string()).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rgpt_types::completion::Message;

    use crate::config::Config;

    use super::*;

    fn get_config() -> Config {
        Config {
            messages: Some(vec![
                Message {
                    role: "system".to_string(),
                    content: "You are my testing assistant. Whatever you say, start with 'Testing: '".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: "Your responses must be short and concise. Do not include explanations unless asked.".to_string(),
                },
                Message {
                    role: "assistant".to_string(),
                    content: "Understood.".to_string(),
                },
            ]),
            ..Default::default()
        }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_session() -> Result<(), Box<dyn std::error::Error>> {
        let cfg = get_config();
        let assistant = Assistant::new(cfg)?;
        let (mut session, input_tx, mut output_rx, kill_tx) = SessionInner::new(assistant);

        tokio::spawn(async move {
            input_tx
                .send("Hello, give me something to think about".to_string())
                .await
                .unwrap();
            input_tx.send("exit".to_string()).await.unwrap();
        });

        tokio::spawn(async move {
            if let Err(e) = session.run().await {
                tracing::error!("error: {}", e);
            }
        });

        let output = output_rx.recv().await.unwrap();
        tracing::debug!("output: {}", output);
        Ok(())
    }
}
