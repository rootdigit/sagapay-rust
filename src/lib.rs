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
//! [`WebhookHandler`] parses incoming notifications and, when you have been
//! issued a platform IPN secret, verifies their `X-Sagapay-Signature` header.
//! [`Client::verify_ipn`] is the primary check: it asks the API to confirm the
//! transaction. Notifications are delivered at least once, so keep your handler
//! idempotent.

mod client;
mod config;
mod error;
mod models;
mod webhook;

pub use client::Client;
pub use config::Config;
pub use error::Error;
pub use models::*;
pub use webhook::WebhookHandler;
