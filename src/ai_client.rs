//! AI Client module - HTTP client for chat API.
//!
//! RUST NOTE: The ChatClient trait allows tests to substitute a fake.
//! All API key handling must never log or print the key.

use crate::model::AiConfig;
use serde::{Deserialize, Serialize};

/// Chat completion request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub max_tokens: u32,
}

/// Chat completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
}

/// Chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
}

/// Trait for chat API clients.
/// RUST NOTE: Tests use a mock implementation.
#[async_trait::async_trait]
pub trait ChatClient: Send + Sync {
    /// Send a chat completion request.
    async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse>;

    /// Get available models.
    async fn list_models(&self) -> anyhow::Result<Vec<String>>;

    /// Probe a model with a tiny test request.
    async fn probe_model(&self, model: &str) -> anyhow::Result<bool>;
}

/// Real HTTP client implementation using blocking reqwest.
pub struct HttpChatClient {
    config: AiConfig,
    client: reqwest::blocking::Client,
}

impl HttpChatClient {
    pub fn new(config: &AiConfig) -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(config.read_timeout_secs))
            .connect_timeout(std::time::Duration::from_secs(config.connect_timeout_secs))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        Ok(Self {
            config: config.clone(),
            client,
        })
    }

    /// Make synchronous chat call with retries.
    pub fn chat_sync(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let mut retries = 0;
        let max_retries = self.config.max_retries;
        
        loop {
            let result = self.send_chat_request(request);
            
            match result {
                Ok(response) => return Ok(response),
                Err(e) if retries < max_retries as usize => {
                    // Exponential backoff
                    let delay = std::time::Duration::from_millis(100 * 2u64.pow(retries as u32));
                    std::thread::sleep(delay);
                    retries += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn send_chat_request(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        // Stub - implementation in M2
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ChatClient for HttpChatClient {
    async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        // Blocking call wrapped in async
        self.chat_sync(request)
    }

    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        // Stub - implementation in M2
        unimplemented!()
    }

    async fn probe_model(&self, _model: &str) -> anyhow::Result<bool> {
        // Stub - implementation in M2
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub() {
        // Stub test
    }
}