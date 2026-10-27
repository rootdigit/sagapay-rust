//! Request headers and redirect handling, against a local one-shot server.

use sagapay::{Client, Config, CreateWithdrawalParams, Error, NetworkType};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

/// Serve one request with `response`, and send back the request's head
/// (request line and headers, lower-cased).
fn serve_once(response: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(value) = lower.strip_prefix("content-length:") {
                content_length = value.trim().parse().unwrap();
            }
            head.push_str(&lower);
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        reader.get_mut().write_all(response.as_bytes()).unwrap();
        tx.send(head).unwrap();
    });
    (base_url, rx)
}

const CREATED: &str = "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: 41\r\nConnection: close\r\n\r\n{\"id\":\"w-1\",\"status\":\"PENDING\",\"fee\":\"0\"}";

fn client(base_url: String) -> Client {
    Client::new(Config {
        api_key: "key".to_string(),
        api_secret: "secret".to_string(),
        base_url,
    })
}

fn withdrawal() -> CreateWithdrawalParams {
    CreateWithdrawalParams {
        network_type: NetworkType::BEP20,
        contract_address: "0".to_string(),
        address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
        amount: "0.5".to_string(),
        ipn_url: "https://example.com/ipn".to_string(),
        udf: None,
    }
}

#[tokio::test]
async fn create_withdrawal_idempotent_sends_the_key() {
    let (base_url, head) = serve_once(CREATED);

    let result = client(base_url)
        .create_withdrawal_idempotent(withdrawal(), "payout-123")
        .await
        .unwrap();

    assert_eq!(result.id, "w-1");
    let head = head.recv().unwrap();
    assert!(head.contains("idempotency-key: payout-123\r\n"), "{head}");
}

#[tokio::test]
async fn create_withdrawal_sends_no_key() {
    let (base_url, head) = serve_once(CREATED);

    client(base_url)
        .create_withdrawal(withdrawal())
        .await
        .unwrap();

    assert!(!head.recv().unwrap().contains("idempotency-key"));
}

#[tokio::test]
async fn redirects_are_not_followed() {
    // Only one request is served: following the redirect would hang or fail
    // to connect rather than return the 307.
    let (base_url, _head) = serve_once(
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:9/steal\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );

    let result = client(base_url).create_withdrawal(withdrawal()).await;

    assert!(
        matches!(
            result,
            Err(Error::ApiError {
                status_code: 307,
                ..
            })
        ),
        "{result:?}"
    );
}
