use sagapay::{Client, Config, Error, IpnType, TransactionStatus, VerifyIpnParams, WebhookHandler};
use std::collections::HashMap;

// This example shows how to handle a webhook (IPN) in a server context.
// This is a simplified example - in a real implementation you would
// integrate this with your web framework (e.g., actix-web, axum, rocket, etc.)

const SAMPLE_BODY: &str = r#"{
  "id": "8b7c1f2e-0f1a-4a4c-9c6b-1f0c7d5a2b3e",
  "type": "DEPOSIT",
  "status": "COMPLETED",
  "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
  "networkType": "BEP20",
  "amount": "1.5",
  "udf": "order-123",
  "txHash": "0xabc123",
  "timestamp": "2025-03-16T14:30:00.000Z"
}"#;

#[tokio::main]
async fn main() {
    println!("This is an example webhook handler implementation.");
    println!("In a real app, integrate this with your web framework.");
    println!();

    // Without a platform-issued IPN secret the handler only parses the body:
    // signature verification is skipped and no signature header is required.
    let parse_only = WebhookHandler::new(None);
    let payload = parse_only
        .process_webhook(SAMPLE_BODY, None)
        .expect("sample payload should parse");

    println!("Parsed sample payload:");
    println!("  ID: {}", payload.id);
    println!("  Type: {:?}", payload.transaction_type); // DEPOSIT / WITHDRAWAL
    println!("  Status: {:?}", payload.status);
    println!("  Amount: {}", payload.amount);
    println!();
    println!("Parsing alone proves nothing - always confirm with verify_ipn.");

    // The handler below is what you would call from your HTTP route. It talks
    // to the API, so it only runs when you ask for it explicitly.
    if std::env::var("SAGAPAY_RUN_HANDLER").is_ok() {
        let client = Client::new(Config {
            api_key: "your-api-key".to_string(),
            api_secret: "your-api-secret".to_string(),
            ..Config::default()
        });

        let mut headers = HashMap::new();
        headers.insert(
            "X-Sagapay-Signature".to_string(),
            "sha256=<hex digest>".to_string(),
        );

        if let Err(err) = handle_webhook(&client, headers, SAMPLE_BODY.to_string()).await {
            eprintln!("Webhook error: {}", err);
        }
    }
}

// Example handler for webhooks in an HTTP server context
async fn handle_webhook(
    client: &Client,
    headers: HashMap<String, String>,
    body: String,
) -> Result<(), Error> {
    // The IPN secret is issued by SagaPay for signing notifications. It is NOT
    // your API secret. Use `None` if you have not been issued one - the
    // signature is then skipped and verify_ipn below does the real checking.
    let ipn_secret: Option<String> = std::env::var("SAGAPAY_IPN_SECRET").ok();
    let webhook_handler = WebhookHandler::new(ipn_secret);

    // Look up the signature header (matched case-insensitively) and validate
    let webhook_data = webhook_handler.process_webhook_from_request(&headers, &body)?;

    // Extract important fields
    let transaction_id = &webhook_data.id;
    let transaction_type = &webhook_data.transaction_type;
    let status = &webhook_data.status;
    let address = &webhook_data.address;
    let amount = &webhook_data.amount;
    let udf = webhook_data.udf.as_deref(); // Your custom reference field
    let tx_hash = webhook_data.tx_hash.as_deref();

    // Log the webhook for debugging
    println!("SagaPay Webhook received:");
    println!("  Transaction ID: {}", transaction_id);
    println!("  Type: {:?}", transaction_type);
    println!("  Status: {:?}", status);
    println!("  Address: {}", address);
    println!("  Amount: {}", amount);

    if let Some(udf) = udf {
        println!("  UDF: {}", udf);
    }

    if let Some(hash) = tx_hash {
        println!("  Transaction Hash: {}", hash);
    }

    // Primary check: ask the API to confirm the notification before acting on
    // it. Notifications are delivered at least once, so make the work below
    // idempotent - key it on the transaction ID.
    if let Some(hash) = tx_hash {
        let verification = client
            .verify_ipn(VerifyIpnParams {
                txn_hash: hash.to_string(),
                ipn_type: transaction_type.clone(),
                amount: amount.clone(),
                address: address.clone(),
            })
            .await?;

        if !verification.verified {
            println!("Verification failed - ignoring this notification.");
            return Ok(());
        }

        println!("  Verified: {}", verification.verified);
    }

    // Handle different transaction statuses. Today SagaPay only sends
    // COMPLETED notifications, but the payload can carry any status.
    match status {
        TransactionStatus::Completed => {
            // Payment successful, update your database and trigger necessary actions
            println!("Transaction completed successfully!");

            // Here you would update your database or trigger business logic
            // based on the transaction type and your custom reference (UDF)
            match transaction_type {
                IpnType::Deposit => {
                    // Handle successful deposit
                    println!("Deposit completed: {} received at {}", amount, address);
                    // Example: update_order_status(udf, "paid");
                }
                IpnType::Withdrawal => {
                    // Handle successful withdrawal
                    println!("Withdrawal completed: {} sent to {}", amount, address);
                    // Example: update_withdrawal_status(udf, "completed");
                }
            }
        }

        TransactionStatus::Failed => {
            // Handle failed transaction
            println!("Transaction failed: {:?} for {}", transaction_type, amount);
            // Example: update_transaction_status(udf, "failed");
        }

        TransactionStatus::Pending | TransactionStatus::Processing => {
            // Handle pending or processing transaction
            println!(
                "Transaction is {:?}: {:?} for {}",
                status, transaction_type, amount
            );
            // Example: update_transaction_status(udf, format!("{:?}", status).to_lowercase());
        }

        TransactionStatus::Cancelled => {
            // Handle cancelled transaction
            println!(
                "Transaction cancelled: {:?} for {}",
                transaction_type, amount
            );
            // Example: update_transaction_status(udf, "cancelled");
        }
    }

    // Return success
    Ok(())
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
//     match handle_webhook(&client, headers, body).await {
//         Ok(_) => HttpResponse::Ok().json(json!({"received": true})),
//         Err(e) => {
//             eprintln!("Webhook error: {:?}", e);
//             // Still return 200 to prevent retries
//             HttpResponse::Ok().json(json!({
//                 "received": false,
//                 "error": e.to_string()
//             }))
//         }
//     }
// }
