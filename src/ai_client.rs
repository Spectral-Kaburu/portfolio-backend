//! AI Client module - HTTP client for chat API.
//!
//! RUST NOTE: The ChatClient trait allows tests to substitute a fake.
//! All API key handling must never log or print the key.

use crate::model::AiConfig;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Chat completion request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub max_tokens: u32,
}

/// Chat completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
}

/// Chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
}

/// Model list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ModelList {
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ModelInfo {
    pub id: String,
}

/// Trait for chat API clients.
/// RUST NOTE: Tests use a mock implementation.
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait ChatClient: Send + Sync {
    /// Send a chat completion request.
    async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse>;

    /// Get available models.
    async fn list_models(&self) -> anyhow::Result<Vec<String>>;

    /// Probe a model with a tiny test request.
    async fn probe_model(&self, model: &str) -> anyhow::Result<bool>;
}

/// Clock trait for injectable time dependency in throttle.
/// RUST NOTE: This abstraction enables testing time-based logic without real delays.
#[allow(dead_code)]
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// System clock implementation.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Request throttler based on requests per minute.
/// RUST NOTE: Thread-safe with Arc<Mutex> for shared state across concurrent requests.
#[allow(dead_code)]
pub struct Throttle {
    requests_per_minute: u32,
    last_request: Arc<Mutex<DateTime<Utc>>>,
    clock: Arc<dyn Clock>,
}

#[allow(dead_code)]
impl Throttle {
    /// Create a new throttler with the given requests per minute.
    pub fn new(requests_per_minute: u32, clock: Arc<dyn Clock>) -> Self {
        Self {
            requests_per_minute,
            last_request: Arc::new(Mutex::new(Utc::now())),
            clock,
        }
    }

    /// Wait if necessary to maintain the rate limit, then return delay.
    pub fn wait(&self) -> Duration {
        let min_interval =
            ChronoDuration::milliseconds((60.0 / self.requests_per_minute as f64 * 1000.0) as i64);

        let mut last = self.last_request.lock().unwrap();
        let now = self.clock.now();
        let elapsed = now - *last;

        if elapsed < min_interval {
            let wait_time = min_interval - elapsed;
            std::thread::sleep(Duration::from_millis(wait_time.num_milliseconds() as u64));
            *last = self.clock.now();
            Duration::from_millis(wait_time.num_milliseconds() as u64)
        } else {
            *last = now;
            Duration::ZERO
        }
    }

    /// Calculate wait time without actually sleeping (for testing).
    pub fn calculate_wait(&self, last_request: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
        let min_interval =
            ChronoDuration::milliseconds((60.0 / self.requests_per_minute as f64 * 1000.0) as i64);
        let elapsed = now - last_request;

        if elapsed < min_interval {
            Duration::from_millis((min_interval - elapsed).num_milliseconds() as u64)
        } else {
            Duration::ZERO
        }
    }
}

/// Real HTTP client implementation using blocking reqwest.
#[allow(dead_code)]
pub struct HttpChatClient {
    config: AiConfig,
    client: reqwest::blocking::Client,
    throttle: Throttle,
}

#[allow(dead_code)]
impl HttpChatClient {
    /// Create a new HTTP chat client with the given configuration.
    pub fn new(config: &AiConfig) -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.read_timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let clock = Arc::new(SystemClock);
        let throttle = Throttle::new(config.requests_per_minute, clock);

        Ok(Self {
            config: config.clone(),
            client,
            throttle,
        })
    }

    /// Create a client with injected clock for testing.
    #[cfg(test)]
    pub fn new_with_clock(config: &AiConfig, clock: Arc<dyn Clock>) -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.read_timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let throttle = Throttle::new(config.requests_per_minute, clock.clone());

        Ok(Self {
            config: config.clone(),
            client,
            throttle,
        })
    }

    /// Make synchronous chat call with retries.
    pub fn chat_sync(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        self.throttle.wait();
        self.send_chat_request_with_retry(request)
    }

    fn send_chat_request_with_retry(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let mut retries = 0u32;
        let max_retries = self.config.max_retries;

        loop {
            let result = self.send_chat_request(request);

            match result {
                Ok(response) => return Ok(response),
                Err(e) if retries < max_retries && Self::is_retryable(&e) => {
                    // Exponential backoff with jitter
                    let base_delay_ms = 100 * 2u64.pow(retries);
                    let jitter = rand::thread_rng().gen_range(0..100);
                    let delay_ms = base_delay_ms + jitter;

                    std::thread::sleep(Duration::from_millis(delay_ms));
                    retries += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn is_retryable(error: &anyhow::Error) -> bool {
        error.to_string().contains("Retryable API error")
    }

    fn send_chat_request(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let api_key = self.config.api_key()?;

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        let status = response.status().as_u16();

        // 401/403 stops immediately - key problem
        if status == 401 {
            return Err(anyhow::anyhow!(
                "Unauthorized: API key may be invalid or missing"
            ));
        }

        if status == 403 {
            return Err(anyhow::anyhow!(
                "Forbidden: API key may be invalid or missing"
            ));
        }

        // 404 on chat suggests "ai models"
        if status == 404 {
            return Err(anyhow::anyhow!(
                "Model not found. Check available AI models with `ai models` command"
            ));
        }

        // 429 and 5xx are retried
        if status == 429 || (500..600).contains(&status) {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            return Err(anyhow::anyhow!(
                "Retryable API error: status {} (retry_after: {:?})",
                status,
                retry_after
            ));
        }

        match response.json::<ChatResponse>() {
            Ok(resp) => Ok(resp),
            Err(e) => Err(anyhow::anyhow!("Failed to parse response: {}", e)),
        }
    }
}

#[async_trait::async_trait]
impl ChatClient for HttpChatClient {
    async fn chat(&self, request: &ChatRequest) -> anyhow::Result<ChatResponse> {
        self.chat_sync(request)
    }

    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        self.throttle.wait();

        let url = format!("{}/models", self.config.base_url);

        let api_key = self.config.api_key()?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if response.status().as_u16() == 401 {
            return Err(anyhow::anyhow!("Unauthorized: API key may be invalid"));
        }

        let model_list: ModelList = response
            .json()
            .map_err(|e| anyhow::anyhow!("Failed to parse model list: {}", e))?;
        Ok(model_list.data.into_iter().map(|m| m.id).collect())
    }

    async fn probe_model(&self, model: &str) -> anyhow::Result<bool> {
        match self.list_models().await {
            Ok(models) => Ok(models.iter().any(|m| m == model)),
            Err(_) => {
                let request = ChatRequest {
                    model: model.to_string(),
                    messages: vec![ChatMessage {
                        role: "user".to_string(),
                        content: "test".to_string(),
                    }],
                    temperature: 0.0,
                    max_tokens: 1,
                };

                match self.chat(&request).await {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, TimeZone, Utc};

    #[test]
    fn test_throttle_calculate_wait_no_wait() {
        let clock = Arc::new(MockClock::new(Utc::now()));
        let throttle = Throttle::new(10, clock.clone());

        let last = Utc::now() - ChronoDuration::minutes(2);
        let now = Utc::now();

        let wait = throttle.calculate_wait(last, now);
        assert_eq!(wait, Duration::ZERO);
    }

    #[test]
    fn test_throttle_calculate_wait_wait() {
        let clock = Arc::new(MockClock::new(Utc::now()));
        let throttle = Throttle::new(10, clock.clone());

        let last = Utc::now() - ChronoDuration::seconds(3);
        let now = Utc::now();

        let wait = throttle.calculate_wait(last, now);
        assert!(
            wait.as_secs() >= 2 && wait.as_secs() <= 4,
            "Expected ~3 seconds wait, got {:?}",
            wait
        );
    }

    #[derive(Debug, Clone)]
    struct MockClock {
        now: DateTime<Utc>,
    }

    impl MockClock {
        fn new(now: DateTime<Utc>) -> Self {
            Self { now }
        }
    }

    impl Clock for MockClock {
        fn now(&self) -> DateTime<Utc> {
            self.now
        }
    }
}
