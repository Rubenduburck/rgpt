use rgpt_types::message::Message;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

use crate::{error::Error, Assistant};

pub type UserInputTx = tokio::sync::mpsc::Sender<String>;
pub type UserInputRx = tokio::sync::mpsc::Receiver<String>;
pub type SystemOutputRx = tokio::sync::mpsc::Receiver<String>;
pub type SystemOutputTx = tokio::sync::mpsc::Sender<String>;
pub type UserKillTx = tokio::sync::mpsc::Sender<()>;
pub type UserKillRx = tokio::sync::mpsc::Receiver<()>;

pub type TaskHandle = tokio::task::JoinHandle<()>;

pub struct Session {
    inner: SessionInner,
    kill_txs: Vec<UserKillTx>,
    input_tx: UserInputTx,
    _cancel_tx: UserKillTx,
}

#[macro_export]
macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

impl Session {
    pub fn setup(assistant: Assistant) -> Result<Self, Error> {
        let (inner, input_tx, output_rx, cancel_tx) = SessionInner::new(assistant);

        let mut kill_txs = Vec::new();

        let (kill_tx, kill_rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(enclose! {(input_tx) async move {
            if let Err(e) = Self::handle_user_input(input_tx, kill_rx).await {
                tracing::error!("error: {}", e)
            }
        }});
        kill_txs.push(kill_tx);

        let (kill_tx, kill_rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            if let Err(e) = Self::handle_output(output_rx, kill_rx).await {
                tracing::error!("error: {}", e)
            }
        });
        kill_txs.push(kill_tx);

        Ok(Session {
            inner,
            kill_txs,
            input_tx,
            _cancel_tx: cancel_tx,
        })
    }

    #[tracing::instrument(skip(input_tx, kill_rx))]
    pub async fn handle_user_input(
        input_tx: UserInputTx,
        mut kill_rx: UserKillRx,
    ) -> Result<(), Error> {
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut line = String::new();
        loop {
            line.clear();
            tokio::select! {
                _ = kill_rx.recv() => {
                    tracing::debug!("killed");
                    return Ok(());
                }
                new_line = reader.read_line(&mut line) => {
                    match new_line {
                        Ok(0) => return Ok(()),
                        Ok(_) => {
                            let line = line.trim().to_string();
                            if line.is_empty() {
                                continue;
                            }
                            input_tx.send(line).await.unwrap();
                        }
                        Err(e) => {
                            tracing::error!("error: {}", e);
                            return Err(Error::Io(e));
                        }
                    }
                }
            }
        }
    }

    #[tracing::instrument(skip(output_rx, kill_rx))]
    pub async fn handle_output(
        mut output_rx: SystemOutputRx,
        mut kill_rx: UserKillRx,
    ) -> Result<(), Error> {
        let mut stdout = tokio::io::stdout();
        loop {
            tokio::select! {
                _ = kill_rx.recv() => {
                    tracing::debug!("killed");
                    return Ok(());
                }
                output = output_rx.recv() => {
                    if let Some(output) = output {
                        stdout.write_all(output.as_bytes()).await?;
                        stdout.flush().await?;
                    } else {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub async fn start(&mut self, messages: &[Message]) -> Result<(), Error> {
        for message in messages {
            if let Err(e) = self.input_tx.send(message.content.clone()).await {
                tracing::error!("error: {}", e);
                return Err(Error::SendInput);
            }
        }
        self.inner.run().await?;
        self.cleanup().await
    }

    pub async fn run_once(&mut self, messages: &[Message]) -> Result<(), Error> {
        for message in messages {
            tracing::debug!("sending message: {}", message.content);
            if let Err(e) = self.input_tx.send(message.content.clone()).await {
                tracing::error!("error: {}", e);
                return Err(Error::SendInput);
            }
        }
        tracing::debug!("running once");
        self.inner.run_once().await?;
        tracing::debug!("cleaning up");
        self.cleanup().await
    }

    pub async fn cleanup(&mut self) -> Result<(), Error> {
        tracing::debug!("cleaning up");
        for kill_tx in self.kill_txs.drain(..) {
            let _ = kill_tx.send(()).await;
        }
        Ok(())
    }
}

pub struct SessionInner {
    input_rx: UserInputRx,
    output_tx: SystemOutputTx,
    cancel_rx: UserKillRx,
    assistant: Assistant,
    buffer: Vec<Message>,
}

impl SessionInner {
    fn new(assistant: Assistant) -> (Self, UserInputTx, SystemOutputRx, UserKillTx) {
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(100);
        let (output_tx, output_rx) = tokio::sync::mpsc::channel(100);
        let (cancel_tx, cancel_rx) = tokio::sync::mpsc::channel(1);

        let session = SessionInner {
            input_rx,
            output_tx,
            cancel_rx,
            assistant,
            buffer: vec![],
        };

        (session, input_tx, output_rx, cancel_tx)
    }

    async fn draw(&mut self, event: &rgpt_types::completion::TextEvent) -> Result<(), Error> {
        if let Some(text) = event.text() {
            self.output_tx
                .send(text.clone())
                .await
                .map_err(|_| Error::SendOutput)?;
        }
        if event.is_stop() {
            self.output_tx
                .send("\n".to_string())
                .await
                .map_err(|_| Error::SendOutput)?;
        }
        Ok(())
    }

    async fn handle_input(&mut self, input: String) -> Result<(), Error> {
        if input == "exit" {
            return Err(Error::Exit);
        }
        self.buffer.push(input.into());

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        self.assistant.handle_input(self.buffer.clone(), tx);
        self.buffer.clear();
        'outer: loop {
            tokio::select! {
                _ = self.cancel_rx.recv() => {
                    tracing::debug!("Cancelled");
                    return Err(Error::Exit);
                }
                event = rx.recv() => {
                    if let Some(event) = event {
                        self.draw(&event).await?;
                        if event.is_complete() {
                            tracing::debug!("completed");
                            break 'outer;
                        }
                    } else {
                        break 'outer;
                    }
                }
            }
        }
        Ok(())
    }

    async fn run_once(&mut self) -> Result<(), Error> {
        if let Some(input) = self.input_rx.recv().await {
            self.handle_input(input).await?;
        }
        Ok(())
    }

    async fn run(&mut self) -> Result<(), Error> {
        while let Some(input) = self.input_rx.recv().await {
            match self.handle_input(input).await {
                Ok(()) => {}
                Err(Error::Exit) => break,
                Err(e) => return Err(e),
            }
            self.output_tx
                .send("> ".to_string())
                .await
                .map_err(|_| Error::SendOutput)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rgpt_types::message::Message;

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
        let (mut session, input_tx, mut output_rx, _kill_tx) = SessionInner::new(assistant);

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
