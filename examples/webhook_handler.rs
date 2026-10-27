use hmac::{Hmac, KeyInit, Mac};
use sagapay::{DEFAULT_MAX_AGE, Error, IpnType, SIGNATURE_HEADER, WebhookHandler, WebhookPayload};
use sha2::Sha256;
use std::collections::{HashMap, HashSet};

// This example shows how to handle a webhook (IPN) in a server context.
// This is a simplified example - in a real implementation you would
// integrate this with your web framework (e.g., actix-web, axum, rocket, etc.)

const SAMPLE_BODY: &str = r#"{
  "id": "8b7c1f2e-0f1a-4a4c-9c6b-1f0c7d5a2b3e",
  "type": "DEPOSIT",
  "status": "COMPLETED",
  "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
  "networkType": "BEP20",
  "asset": "USDT",
  "contractAddress": "0x55d398326f99059fF775485246999027B3197955",
  "amount": "1.5",
  "udf": "order-123",
  "txHash": "0xabc123",
  "timestamp": "2025-03-16T14:30:00.000Z"
}"#;

// Stand-in for your signing secret (dashboard → Webhooks → Signing secret).
// Load the real one from configuration; never commit it.
const DEMO_SECRET: &str = "demo-signing-secret";

fn main() {
    println!("This is an example webhook handler implementation.");
    println!("In a real app, integrate this with your web framework.");
    println!();

    let signing_secret =
        std::env::var("SAGAPAY_SIGNING_SECRET").unwrap_or_else(|_| DEMO_SECRET.to_string());
    let webhook_handler = WebhookHandler::new(signing_secret.clone());

    // SagaPay sends this header with every notification. Here we sign the
    // sample body ourselves so the example runs without a server.
    let mut headers = HashMap::new();
    headers.insert(
        SIGNATURE_HEADER.to_string(),
        sign(&signing_secret, SAMPLE_BODY),
    );

    // IDs already handled. Use a database table with a unique column in
    // production; this set is lost on restart.
    let mut processed = HashSet::new();

    // Delivering the same notification twice acts on it once.
    for _ in 0..2 {
        match handle_webhook(&webhook_handler, &headers, SAMPLE_BODY, &mut processed) {
            Ok(()) => {}
            Err(err) => eprintln!("Rejected webhook: {}", err),
        }
    }

    // A tampered body fails the signature check.
    let tampered = SAMPLE_BODY.replace("\"1.5\"", "\"1500\"");
    if let Err(err) = handle_webhook(&webhook_handler, &headers, &tampered, &mut processed) {
        println!("Tampered body rejected: {}", err);
    }

    // With a maximum age, the 2025 sample is refused as stale.
    let strict = WebhookHandler::new(signing_secret).with_max_age(DEFAULT_MAX_AGE);
    if let Err(err) = handle_webhook(&strict, &headers, SAMPLE_BODY, &mut HashSet::new()) {
        println!("Old notification rejected: {}", err);
    }
}

// What you would call from your HTTP route. Answer 401 when it returns an
// error, and 200 otherwise; if your own processing fails, answer 5xx so
// SagaPay retries.
fn handle_webhook(
    webhook_handler: &WebhookHandler,
    headers: &HashMap<String, String>,
    body: &str,
    processed: &mut HashSet<String>,
) -> Result<(), Error> {
    // Looks up X-Sagapay-Merchant-Signature (matched case-insensitively),
    // verifies it over the exact raw body, then parses the body.
    let webhook = webhook_handler.process_webhook_from_request(headers, body)?;

    // Delivery is at-least-once, and a replayed request carries a valid
    // signature too, so act on each ID only once.
    if !processed.insert(webhook.id.clone()) {
        println!("Duplicate notification {} - already handled.", webhook.id);
        return Ok(());
    }

    fulfil(&webhook);
    Ok(())
}

fn fulfil(webhook: &WebhookPayload) {
    println!("SagaPay Webhook received:");
    println!("  Transaction ID: {}", webhook.id);
    println!("  Type: {:?}", webhook.transaction_type);
    println!("  Asset: {:?}", webhook.asset);
    println!("  Amount: {}", webhook.amount);

    // Status is currently always COMPLETED.
    match webhook.transaction_type {
        IpnType::Deposit => {
            // Check asset, contract_address and amount against the order
            // before you mark it paid.
            println!(
                "Deposit completed: {} received at {}",
                webhook.amount, webhook.address
            );
            // Example: update_order_status(webhook.udf, "paid");
        }
        IpnType::Withdrawal => {
            println!(
                "Withdrawal completed: {} sent to {}",
                webhook.amount, webhook.address
            );
            // Example: update_withdrawal_status(webhook.udf, "completed");
        }
    }
}

/// Sign a body the way SagaPay does: "sha256=" + hex HMAC-SHA256
fn sign(secret: &str, body: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key size");
    mac.update(body.as_bytes());
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

// Example of how to use this in a web server (pseudocode)
//
// In Actix-web:
//
// #[post("/webhook")]
// async fn webhook_endpoint(req: HttpRequest, body: String) -> impl Responder {
//     // Convert headers to HashMap (casing does not matter)
//     let mut headers = HashMap::new();
//     for (name, value) in req.headers() {
//         if let Ok(value_str) = value.to_str() {
//             headers.insert(name.as_str().to_string(), value_str.to_string());
//         }
//     }
//
//     match handle_webhook(&webhook_handler, &headers, &body, &mut processed) {
//         Ok(_) => HttpResponse::Ok().json(json!({"received": true})),
//         Err(_) => HttpResponse::Unauthorized().finish(),
//     }
// }
