# SagaPay Rust SDK

Rust SDK for [SagaPay](https://sagapay.io) - the world's first free, non-custodial blockchain payment gateway service provider. This library enables Rust developers to seamlessly integrate cryptocurrency payments without holding customer funds. With enterprise-grade security and zero transaction fees, SagaPay empowers merchants to accept crypto payments across multiple blockchains while maintaining full control of their digital assets.

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
sagapay = "0.3.0"
tokio = { version = "1", features = ["full"] }
```

The crate is published and imported as `sagapay`.

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

Withdrawals need an API key created with **Withdrawals** turned on, which requires two-factor authentication on your account.

```rust
let withdrawal = client.create_withdrawal_idempotent(CreateWithdrawalParams {
    network_type: NetworkType::ERC20,
    contract_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT
    address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
    amount: "10.5".to_string(),
    ipn_url: "https://example.com/webhook".to_string(),
    udf: Some("withdrawal-456".to_string()),
}, "payout-456").await?;
```

`create_withdrawal_idempotent` sends the key (1-128 letters, digits, `-` or `_`) as an `Idempotency-Key` header, so that a retry after a timeout or network error cannot send the money twice. For 24 hours, a repeat with the same key returns the first withdrawal instead of creating another; while the first request is still running, a repeat fails with a 409 `Error::ApiError`. Use one key per payout, such as your own payout id. `create_withdrawal` takes the same parameters without a key.

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
    // The API confirms the transaction
}
```

Check the webhook signature first (see below); this network call is an extra check on top of it.

The API key and secret are read from the client `Config` and sent in the request
body - `/verify-ipn` is the one endpoint that takes no auth headers.

## Handling Webhooks (IPN)

SagaPay sends webhook notifications to your specified `ipnUrl` when a transaction completes. Each one carries a signature over the exact raw request body:

```
X-Sagapay-Merchant-Signature: sha256=<hex HMAC-SHA256 of the raw body>
```

It is keyed with your **signing secret**. Copy it from the dashboard: **Webhooks → Signing secret**. It is not your API secret, and nobody else's secret can produce a valid signature for your notifications. `WebhookHandler` checks the signature on every notification, and a handler created with an empty secret rejects them all.

```rust
use sagapay::{DEFAULT_MAX_AGE, Error, IpnType, WebhookHandler};
use std::collections::HashMap;

// In your webhook endpoint handler. Answer 401 on Err; answer 2xx within 20
// seconds otherwise. If your own processing fails, answer 5xx so SagaPay retries.
fn handle_webhook(headers: &HashMap<String, String>, body: &str) -> Result<(), Error> {
    // Webhooks → Signing secret. Optional: also refuse notifications older than a day.
    let handler = WebhookHandler::new(std::env::var("SAGAPAY_SIGNING_SECRET").unwrap_or_default())
        .with_max_age(DEFAULT_MAX_AGE);

    // Looks up X-Sagapay-Merchant-Signature (case-insensitively), verifies it
    // over the exact raw body, then parses the body.
    let webhook = handler.process_webhook_from_request(headers, body)?;

    // Process each id once. Delivery is at-least-once, and a replayed request
    // carries a valid signature too.
    if !mark_processed(&webhook.id) { // e.g. INSERT ... ON CONFLICT DO NOTHING
        return Ok(());
    }

    match webhook.transaction_type {
        // Check what arrived before you credit it.
        IpnType::Deposit => credit_order(&webhook.udf, &webhook.asset, &webhook.contract_address, &webhook.amount),
        IpnType::Withdrawal => mark_payout_sent(&webhook.udf, &webhook.tx_hash),
    }

    Ok(())
}
```

If you have the header yourself, `handler.process_webhook(body, signature)` takes it as an `Option<&str>`. `sagapay::SIGNATURE_HEADER` holds the header name. Verify against the raw bytes you received: re-serializing the JSON changes the digest. The handler accepts both the `sha256=`-prefixed form and a bare hex digest, and compares in constant time.

The old `X-Sagapay-Signature` header was keyed with a secret shared by every merchant. It is retired: SagaPay no longer sends it, and this SDK no longer reads it.

### Replay protection

A captured webhook still has a valid signature, so the signature alone does not stop someone resending it. Protect against that:

- **Process each `id` once.** Record the ids you have handled (a unique column works) and answer duplicates with a 200 without acting on them again. This is the main defence and also covers normal retries.
- **Optionally check the age** with `with_max_age`, or `sagapay::is_ipn_fresh(&webhook, max_age, Utc::now())`. Keep the limit above the retry schedule (about 7.5 hours): retries reuse the original `timestamp`. A notification you resend from the dashboard also keeps it, so a resend older than your limit will be refused.
- **Check what you are crediting.** Match `asset`, `contract_address`, `amount` and `address` against the order before you fulfil it.

## Webhook Payload Format

When SagaPay sends a webhook to your endpoint, it will include the following payload:

```json
{
  "id": "transaction-uuid",
  "type": "DEPOSIT",
  "status": "COMPLETED",
  "address": "0x123abc...",
  "networkType": "ERC20",
  "asset": "USDT",
  "contractAddress": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
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
- `asset` and `contractAddress` (`contract_address` in Rust) say what was
  received; `contractAddress` is `null` for the chain's native coin.
  `networkType` alone does not tell 100 TRX from 100 USDT.
- `udf` and `txHash` may be `null`.
- `timestamp` is set when the transaction completes and does not change on retries.
- SagaPay makes up to 6 delivery attempts, retrying after 1 min, 5 min, 15 min,
  1 h and 6 h.

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
        Error::StaleWebhook => {
            eprintln!("Webhook older than the handler's maximum age");
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

For questions or support, please contact support@sagapay.io or visit [https://sagapay.io](https://sagapay.io).