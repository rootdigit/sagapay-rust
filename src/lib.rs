//! # SagaPay Rust SDK
//!
//! This crate provides a Rust interface to the SagaPay blockchain payment gateway API.
//! SagaPay is the world's first free, non-custodial blockchain payment gateway service provider.
//!
//! ## Example
//!
//! ```rust,no_run
//! use sagapay::{AddressType, Client, Config, CreateDepositParams, NetworkType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), sagapay::Error> {
//!     // Initialize the SagaPay client
//!     let client = Client::new(Config {
//!         api_key: "your-api-key".to_string(),
//!         api_secret: "your-api-secret".to_string(),
//!         ..Config::default()
//!     });
//!
//!     // Create a deposit address
//!     let deposit = client.create_deposit(CreateDepositParams {
//!         network_type: NetworkType::BEP20,
//!         contract_address: "0".to_string(), // '0' for native tokens (BNB)
//!         amount: "1.5".to_string(),
//!         ipn_url: "https://example.com/webhook".to_string(),
//!         udf: Some("order-123".to_string()),
//!         address_type: Some(AddressType::Temporary),
//!         transfer_balance: None, // defaults to true server side
//!     }).await?;
//!
//!     println!("Deposit address: {}", deposit.address);
//!     println!("Expires at: {:?}", deposit.expires_at);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Notifications (IPN)
//!
//! [`WebhookHandler`] verifies each notification's `X-Sagapay-Merchant-Signature`
//! header with your signing secret (dashboard: Webhooks → Signing secret), then
//! parses it. Notifications are delivered at least once, and a replayed request
//! carries a valid signature too, so act on each `id` only once.
//! [`is_ipn_fresh`] optionally rejects old notifications.

mod client;
mod config;
mod error;
mod models;
mod webhook;

pub use client::Client;
pub use config::Config;
pub use error::Error;
pub use models::*;
pub use webhook::{DEFAULT_MAX_AGE, SIGNATURE_HEADER, WebhookHandler, is_ipn_fresh};
