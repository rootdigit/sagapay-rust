# SagaPay Rust SDK

Rust SDK for [SagaPay](https://sagapay.net) - the world's first free, non-custodial blockchain payment gateway service provider. This library enables Rust developers to seamlessly integrate cryptocurrency payments without holding customer funds. With enterprise-grade security and zero transaction fees, SagaPay empowers merchants to accept crypto payments across multiple blockchains while maintaining full control of their digital assets.

## Features

- Deposit address generation
- Withdrawal processing
- Transaction status checking
- Wallet balance fetching
- Webhook notifications (IPN) with signature checking and server-side verification
- Multi-chain support (ERC20, BEP20, TRC20, POLYGON, SOLANA)
- Custom UDF field support
- Non-custodial architecture
- Async support with tokio

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
sagapay-sdk = "0.2.0"
tokio = { version = "1", features = ["full"] }
```

The crate is published as `sagapay-sdk` and imported as `sagapay`.

Requires Rust 1.85 or newer (edition 2024).

## Quick Start

```rust
use sagapay::{AddressType, Client, Config, CreateDepositParams, NetworkType};

#[tokio::main]
async fn main() -> Result<(), sagapay::Error> {
    // Initialize the SagaPay client
    let client = Client::new(Config {
        api_key: "your-api-key".to_string(),
        api_secret: "your-api-secret".to_string(),
        ..Config::default()
    });

    // Create a deposit address
    let deposit = client.create_deposit(CreateDepositParams {
        network_type: NetworkType::BEP20,
        contract_address: "0".to_string(), // Use '0' for native coins
        amount: "1.5".to_string(),
        ipn_url: "https://yourwebsite.com/webhook".to_string(),
        udf: Some("order-123".to_string()),
        address_type: Some(AddressType::Temporary),
        transfer_balance: None, // Defaults to true server side
    }).await?;

    println!("Deposit address created: {}", deposit.address);
    println!("Expires at: {:?}", deposit.expires_at); // None for permanent addresses

    Ok(())
}
```

## API Reference

### Create Deposit

```rust
let deposit = client.create_deposit(CreateDepositParams {
    network_type: NetworkType::BEP20,               // Required: Blockchain network type
    contract_address: "0".to_string(),              // Required: Contract address or '0' for native coins
    amount: "1.5".to_string(),                      // Required: Expected deposit amount
    ipn_url: "https://example.com/webhook".to_string(), // Required: URL for notifications
    udf: Some("order-123".to_string()),             // Optional: User-defined field
    address_type: Some(AddressType::Temporary),     // Optional: Temporary or Permanent
    transfer_balance: Some(true),                   // Optional: Sweep funds to your wallet, default true
}).await?;
```

### Create Withdrawal

```rust
let withdrawal = client.create_withdrawal(CreateWithdrawalParams {
    network_type: NetworkType::ERC20,
    contract_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT
    address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
    amount: "10.5".to_string(),
    ipn_url: "https://example.com/webhook".to_string(),
    udf: Some("withdrawal-456".to_string()),
}).await?;
```

### Check Transaction Status

```rust
// By address
let status = client.check_transaction_status(
    TransactionType::Deposit,
    Some("0x742d35Cc6634C0532925a3b844Bc454e4438f44e"),
    None,
).await?;

// By transaction ID
let status = client.check_transaction_status(
    TransactionType::Deposit,
    None,
    Some("deposit-uuid"),
).await?;
```

`TransactionType` is the lowercase form used by this query (`deposit` / `withdrawal`).
Deposit items carry `confirmations`, withdrawal items carry `fee` and `processed_at`.

### Fetch Wallet Balance

```rust
// Token balance
let balance = client.fetch_wallet_balance(
    "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    NetworkType::ERC20,
    Some("0xdAC17F958D2ee523a2206206994597C13D831ec7") // USDT on Ethereum
).await?;

// Native coin balance - pass None to omit the contract address
let native = client.fetch_wallet_balance(
    "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    NetworkType::ERC20,
    None
).await?;
```

The response's `contract_address` is `"0"` for native coins. Here `token` carries
only `symbol`, `name` and `decimals`, so `token.network_type` and
`token.contract_address` are `None` - `check_transaction_status` fills them in.

### Verify an IPN

```rust
use sagapay::{IpnType, VerifyIpnParams};

let result = client.verify_ipn(VerifyIpnParams {
    txn_hash: "0xabc123".to_string(),
    ipn_type: IpnType::Deposit, // Serialized as "DEPOSIT"
    amount: "1.5".to_string(),
    address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
}).await?;

if result.verified {
    // Safe to credit the order
}
```

The API key and secret are read from the client `Config` and sent in the request
body - `/verify-ipn` is the one endpoint that takes no auth headers.

## Handling Webhooks (IPN)

SagaPay sends webhook notifications to your specified `ipnUrl` when transaction statuses change. Use the `WebhookHandler` to parse and (optionally) authenticate them:

```rust
use sagapay::{Client, Error, TransactionStatus, VerifyIpnParams, WebhookHandler};
use std::collections::HashMap;

// In your webhook endpoint handler:
async fn handle_webhook(
    client: &Client,
    headers: HashMap<String, String>,
    body: String
) -> Result<(), Error> {
    // The IPN secret is issued by the platform for signing notifications.
    // It is NOT your API secret. Pass None if you were not issued one -
    // signature verification is then skipped and the body is only parsed.
    let ipn_secret: Option<String> = std::env::var("SAGAPAY_IPN_SECRET").ok();
    let handler = WebhookHandler::new(ipn_secret);

    // Looks up X-Sagapay-Signature (case-insensitively) and validates the body
    let webhook_data = handler.process_webhook_from_request(&headers, &body)?;

    // Primary check: ask the API to confirm the notification before acting on it
    if let Some(tx_hash) = webhook_data.tx_hash.as_deref() {
        let result = client.verify_ipn(VerifyIpnParams {
            txn_hash: tx_hash.to_string(),
            ipn_type: webhook_data.transaction_type.clone(),
            amount: webhook_data.amount.clone(),
            address: webhook_data.address.clone(),
        }).await?;

        if !result.verified {
            return Ok(()); // Not a genuine notification - ignore it
        }
    }

    // Handle the verified webhook data based on status
    match webhook_data.status {
        TransactionStatus::Completed => {
            // Payment successful
            // Update your database or trigger actions
        },
        TransactionStatus::Failed => {
            // Handle failed transaction
        },
        _ => { /* Handle other statuses */ }
    }

    Ok(())
}
```

If you also want a lower level entry point, `handler.process_webhook(&body, signature)`
takes the signature as an `Option<&str>`.

### Signature

Signed notifications carry this header:

```
X-Sagapay-Signature: sha256=<hex HMAC-SHA256 of the exact raw request body>
```

The key is the platform-issued IPN secret, not your API secret (SagaPay stores
only a hash of the API secret, so it can never be the signing key). Verify
against the raw bytes you received - re-serializing the JSON changes the digest.
The handler accepts both the `sha256=`-prefixed form and a bare hex digest, and
compares in constant time.

### Delivery guarantees

Notifications are delivered **at least once**, so the same payload can arrive
more than once. Treat the payload `id` as an idempotency key and make your
handler safe to run twice.

## Webhook Payload Format

When SagaPay sends a webhook to your endpoint, it will include the following payload:

```json
{
  "id": "transaction-uuid",
  "type": "DEPOSIT",
  "status": "COMPLETED",
  "address": "0x123abc...",
  "networkType": "ERC20",
  "amount": "10.5",
  "udf": "your-optional-user-defined-field",
  "txHash": "0xabc123...",
  "timestamp": "2025-03-16T14:30:00Z"
}
```

- `type` is uppercase (`DEPOSIT` or `WITHDRAWAL`) and maps to `IpnType`. This is
  distinct from the lowercase `TransactionType` used by `check_transaction_status`.
- `status` is `COMPLETED` - the only status currently notified.
- `networkType` is one of `ERC20`, `BEP20`, `TRC20`, `POLYGON`, `SOLANA`.
- `udf` and `txHash` may be `null`.

## Error Handling

The SDK provides a comprehensive error handling system using the `thiserror` crate:

```rust
match client.create_deposit(params).await {
    Ok(deposit) => {
        println!("Success! Address: {}", deposit.address);
    },
    Err(e) => match e {
        Error::NetworkError(err) => {
            eprintln!("Network error: {}", err);
        },
        Error::ApiError { status_code, message, .. } => {
            eprintln!("API error {}: {}", status_code, message);
        },
        Error::InvalidSignature => {
            eprintln!("Invalid or missing webhook signature");
        },
        _ => {
            eprintln!("Other error: {}", e);
        }
    }
}
```

`ApiError::message` is the human readable text from the response body, which the
API returns as `{ "error": "..." }`.

## License

This SDK is released under the MIT License.

## Support

For questions or support, please contact support@sagapay.net or visit [https://sagapay.net](https://sagapay.net).