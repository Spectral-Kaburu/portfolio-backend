//! AI Client tests with mock HTTP server.
//!
//! RUST NOTE: Uses std::net::TcpListener for in-process mock server.
//! Tests run synchronously without external network calls.

use portfolio_engine::ai_client::{ChatClient, ChatMessage, ChatRequest, HttpChatClient, Throttle};
use portfolio_engine::model::AiConfig;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::thread;
use std::time::Duration;

/// Helper to create a mock HTTP response.
fn make_chunked_response(status: u16, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {} OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n{}",
        status,
        body.len(),
        body
    )
    .into_bytes()
}

fn make_error_response(status: u16) -> Vec<u8> {
    format!("HTTP/1.1 {} Error\r\nContent-Type: text/plain\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", status).into_bytes()
}

fn handle_connection(mut stream: TcpStream, responses: &[Vec<u8>], counter: &AtomicU32) {
    let mut buf = [0; 4096];
    let _ = stream.read(&mut buf);

    let count = counter.fetch_add(1, Ordering::SeqCst);
    let response = responses
        .get(count as usize)
        .unwrap_or(&responses[responses.len() - 1]);

    let _ = stream.write_all(response);
    let _ = stream.flush();
}

fn run_mock_server(listener: TcpListener, responses: Vec<Vec<u8>>, request_count: Arc<AtomicU32>) {
    let responses = Arc::new(responses);
    let counter = Arc::clone(&request_count);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let responses = Arc::clone(&responses);
                let counter = Arc::clone(&counter);
                thread::spawn(move || {
                    handle_connection(stream, &responses, &counter);
                });
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }
}

/// Test: 429 -> 500 -> success yields success (retry logic works)
#[test]
fn test_retry_logic_429_then_500_then_success() {
    unsafe {
        std::env::set_var("NVIDIA_API_KEY", "test-key");
    }

    let request_count = Arc::new(AtomicU32::new(0));
    let request_count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let local_addr = listener.local_addr().expect("Failed to get local address");
    let port = local_addr.port();

    // Spawn server thread that can handle multiple connections
    let responses = vec![
        make_chunked_response(429, r#"{"error": "rate limit"}"#),
        make_chunked_response(500, r#"{"error": "server error"}"#),
        make_chunked_response(
            200,
            r#"{"choices":[{"index":0,"message":{"role":"assistant","content":"ok"}}]}"#,
        ),
    ];

    let server_thread = thread::spawn(move || {
        run_mock_server(listener, responses, request_count_clone);
    });

    thread::sleep(Duration::from_millis(10));

    let mut config = AiConfig::default();
    config.base_url = format!("http://127.0.0.1:{}", port);
    config.max_retries = 3;
    config.requests_per_minute = 100;

    let client = HttpChatClient::new(&config).expect("Failed to create client");

    let request = ChatRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
        }],
        temperature: 0.7,
        max_tokens: 100,
    };

    let result = client.chat_sync(&request);

    // Wait a bit for server to finish
    thread::sleep(Duration::from_millis(100));

    assert!(
        result.is_ok(),
        "Expected success after retries, got: {:?}",
        result
    );
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        3,
        "Expected 3 requests (429 -> 500 -> success)"
    );
}

/// Test: 401 stops immediately (key problem)
#[test]
fn test_401_stops_immediately() {
    unsafe {
        std::env::set_var("NVIDIA_API_KEY", "test-key");
    }

    let request_count = Arc::new(AtomicU32::new(0));
    let request_count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let local_addr = listener.local_addr().expect("Failed to get local address");
    let port = local_addr.port();

    let responses = vec![make_error_response(401)];
    let server_thread = thread::spawn(move || {
        run_mock_server(listener, responses, request_count_clone);
    });

    thread::sleep(Duration::from_millis(10));

    let mut config = AiConfig::default();
    config.base_url = format!("http://127.0.0.1:{}", port);
    config.requests_per_minute = 100;

    let client = HttpChatClient::new(&config).expect("Failed to create client");

    let request = ChatRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
        }],
        temperature: 0.7,
        max_tokens: 100,
    };

    let result = client.chat_sync(&request);

    thread::sleep(Duration::from_millis(100));

    assert!(result.is_err(), "Expected error on 401");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Unauthorized"),
        "Error should mention unauthorized: {}",
        err
    );
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        1,
        "Should only make 1 request on 401"
    );
}

/// Test: 404 on chat suggests "ai models"
#[test]
fn test_404_suggests_ai_models() {
    unsafe {
        std::env::set_var("NVIDIA_API_KEY", "test-key");
    }

    let request_count = Arc::new(AtomicU32::new(0));
    let request_count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let local_addr = listener.local_addr().expect("Failed to get local address");
    let port = local_addr.port();

    let responses = vec![make_error_response(404)];
    let server_thread = thread::spawn(move || {
        run_mock_server(listener, responses, request_count_clone);
    });

    thread::sleep(Duration::from_millis(10));

    let mut config = AiConfig::default();
    config.base_url = format!("http://127.0.0.1:{}", port);
    config.requests_per_minute = 100;

    let client = HttpChatClient::new(&config).expect("Failed to create client");

    let request = ChatRequest {
        model: "nonexistent-model".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
        }],
        temperature: 0.7,
        max_tokens: 100,
    };

    let result = client.chat_sync(&request);

    thread::sleep(Duration::from_millis(100));

    assert!(result.is_err(), "Expected error on 404");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("ai models"),
        "Error should suggest ai models: {}",
        err
    );
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        1,
        "Should only make 1 request on 404"
    );
}

/// Test: throttle is testable as pure function with injected clock
#[test]
fn test_throttle_calculate_wait_is_pure() {
    use chrono::{Duration as ChronoDuration, TimeZone, Utc};

    // Create mock clock at known time
    let base_time = Utc.timestamp_opt(0, 0).single().unwrap(); // Unix epoch
    let clock = Arc::new(TestClock::new(base_time));
    let throttle = Throttle::new(10, clock); // 10 req/min = 6 seconds minimum

    // Test case: request 3 seconds ago (less than min interval)
    let last = base_time;
    let now = base_time + ChronoDuration::seconds(3);

    let wait = throttle.calculate_wait(last, now);

    // Should wait approximately 3 more seconds (6 - 3 = 3)
    assert!(
        wait.as_secs() >= 2 && wait.as_secs() <= 4,
        "Expected ~3 seconds wait, got {:?}",
        wait
    );
}

/// Test: no real network calls - mock server only
#[test]
fn test_no_real_network_calls() {
    unsafe {
        std::env::set_var("NVIDIA_API_KEY", "test-key");
    }

    let request_count = Arc::new(AtomicU32::new(0));
    let request_count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let port = listener.local_addr().unwrap().port();

    let responses = vec![make_chunked_response(
        200,
        r#"{"choices":[{"index":0,"message":{"role":"assistant","content":"ok"}}]}"#,
    )];
    let server_thread = thread::spawn(move || {
        run_mock_server(listener, responses, request_count_clone);
    });

    thread::sleep(Duration::from_millis(10));

    let mut config = AiConfig::default();
    config.base_url = format!("http://127.0.0.1:{}", port);
    config.requests_per_minute = 100;

    let client = HttpChatClient::new(&config).expect("Failed to create client");

    let request = ChatRequest {
        model: "test".to_string(),
        messages: vec![],
        temperature: 0.0,
        max_tokens: 1,
    };

    let _ = client.chat_sync(&request);

    thread::sleep(Duration::from_millis(100));

    // Verify the server received the connection
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        1,
        "Server should have received 1 connection"
    );
}

/// Test: successful chat returns valid response
#[test]
fn test_successful_chat() {
    unsafe {
        std::env::set_var("NVIDIA_API_KEY", "test-key");
    }

    let request_count = Arc::new(AtomicU32::new(0));
    let request_count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
    let port = listener.local_addr().unwrap().port();

    let responses = vec![make_chunked_response(
        200,
        r#"{"choices":[{"index":0,"message":{"role":"assistant","content":"Hello, world!"}}]}"#,
    )];
    let server_thread = thread::spawn(move || {
        run_mock_server(listener, responses, request_count_clone);
    });

    thread::sleep(Duration::from_millis(10));

    let mut config = AiConfig::default();
    config.base_url = format!("http://127.0.0.1:{}", port);
    config.requests_per_minute = 100;

    let client = HttpChatClient::new(&config).expect("Failed to create client");

    let request = ChatRequest {
        model: "test".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        }],
        temperature: 0.7,
        max_tokens: 100,
    };

    let result = client.chat_sync(&request);

    thread::sleep(Duration::from_millis(100));

    assert!(result.is_ok(), "Expected success, got: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.choices[0].message.content, "Hello, world!");
    assert_eq!(request_count.load(Ordering::SeqCst), 1);
}

/// Test clock for injectable time dependency
#[derive(Debug, Clone)]
struct TestClock {
    now: chrono::DateTime<chrono::Utc>,
}

impl TestClock {
    fn new(now: chrono::DateTime<chrono::Utc>) -> Self {
        Self { now }
    }
}

impl portfolio_engine::ai_client::Clock for TestClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.now
    }
}
