use std::pin::Pin;

use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_stream::{Stream, StreamExt};

use super::error::{map_deserialization_error, Error, WrappedError};

#[derive(Debug)]
pub struct Client {
    pub http_client: reqwest::Client,
    pub backoff: backoff::ExponentialBackoff,
    pub headers: HeaderMap,
}

impl Client {
    pub fn new(headers: HeaderMap) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            backoff: Default::default(),
            headers,
        }
    }

    pub async fn post<I, O>(&self, uri: &str, request: I) -> Result<O, Error>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        let request = self
            .http_client
            .post(uri)
            .headers(self.headers.clone())
            .body(serde_json::to_vec(&request)?)
            .build()?;

        self.execute(request).await
    }

    pub async fn post_stream<I, O>(
        &self,
        uri: &str,
        request: I,
    ) -> Pin<Box<dyn Stream<Item = Result<O, Error>> + Send>>
    where
        I: Serialize,
        O: DeserializeOwned + Send + 'static,
    {
        let event_source = self
            .http_client
            .post(uri)
            .headers(self.headers.clone())
            .body(serde_json::to_vec(&request).expect("Failed to serialize request"))
            .eventsource()
            .unwrap();

        stream(event_source).await
    }

    async fn process_response<O>(&self, response: reqwest::Response) -> Result<O, Error>
    where
        O: DeserializeOwned,
    {
        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let wrapped_error: WrappedError = serde_json::from_slice(bytes.as_ref())
                .map_err(|e| map_deserialization_error(e, bytes.as_ref()))?;

            return Err(Error::ApiError(wrapped_error.error));
        }

        let response: O = serde_json::from_slice(bytes.as_ref())
            .map_err(|e| map_deserialization_error(e, bytes.as_ref()))?;
        Ok(response)
    }

    async fn execute<O>(&self, request: reqwest::Request) -> Result<O, Error>
    where
        O: DeserializeOwned,
    {
        let client = self.http_client.clone();

        match request.try_clone() {
            // Only clone-able requests can be retried
            Some(request) => {
                backoff::future::retry(self.backoff.clone(), || async {
                    let response = client
                        .execute(request.try_clone().unwrap())
                        .await
                        .map_err(Error::Reqwest)
                        .map_err(backoff::Error::Permanent)?;

                    let status = response.status();
                    let bytes = response
                        .bytes()
                        .await
                        .map_err(Error::Reqwest)
                        .map_err(backoff::Error::Permanent)?;

                    // Deserialize response body from either error object or actual response object
                    if !status.is_success() {
                        let wrapped_error: WrappedError = serde_json::from_slice(bytes.as_ref())
                            .map_err(|e| map_deserialization_error(e, bytes.as_ref()))
                            .map_err(backoff::Error::Permanent)?;

                        // Retry if rate limited
                        if status.as_u16() == 429 {
                            return Err(backoff::Error::Transient {
                                err: Error::ApiError(wrapped_error.error),
                                retry_after: None,
                            });
                        } else {
                            return Err(backoff::Error::Permanent(Error::ApiError(
                                wrapped_error.error,
                            )));
                        }
                    }

                    let response: O = serde_json::from_slice(bytes.as_ref())
                        .map_err(|e| map_deserialization_error(e, bytes.as_ref()))
                        .map_err(backoff::Error::Permanent)?;
                    Ok(response)
                })
                .await
            }
            None => {
                let response = client.execute(request).await?;
                self.process_response(response).await
            }
        }
    }
}

async fn stream<O>(
    mut event_source: EventSource,
) -> Pin<Box<dyn Stream<Item = Result<O, Error>> + Send>>
where
    O: DeserializeOwned + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(ev) = event_source.next().await {
            match ev {
                Ok(event) => match event {
                    Event::Open => continue,
                    Event::Message(message) => {
                        match message.event.as_ref() {
                            "ping" => continue,
                            "completion" => {
                                let response = match serde_json::from_str::<O>(&message.data) {
                                    Ok(output) => Ok(output),
                                    Err(e) => {
                                        Err(map_deserialization_error(e, message.data.as_bytes()))
                                    }
                                };

                                if let Err(_e) = tx.send(response) {
                                    // rx dropped
                                    break;
                                }
                            }
                            _ => continue,
                        }
                    }
                },
                Err(e) => {
                    if let Err(_e) = tx.send(Err(Error::StreamError(e.to_string()))) {
                        // rx dropped
                        break;
                    }
                }
            }
        }

        event_source.close();
    });

    Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
}
