use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported blockchain network types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NetworkType {
    ERC20,
    BEP20,
    TRC20,
    POLYGON,
    SOLANA,
}

impl fmt::Display for NetworkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkType::ERC20 => write!(f, "ERC20"),
            NetworkType::BEP20 => write!(f, "BEP20"),
            NetworkType::TRC20 => write!(f, "TRC20"),
            NetworkType::POLYGON => write!(f, "POLYGON"),
            NetworkType::SOLANA => write!(f, "SOLANA"),
        }
    }
}

/// Transaction status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// Transaction type enum
///
/// Serialized in lowercase, as expected by the `type` query parameter of
/// `/check-transaction-status`. IPN payloads use [`IpnType`] instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
}

/// Transaction type as used by the IPN endpoints
///
/// Serialized in uppercase (`DEPOSIT` / `WITHDRAWAL`), the shape used by
/// outbound IPN payloads and by the `/verify-ipn` request body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum IpnType {
    Deposit,
    Withdrawal,
}

impl fmt::Display for IpnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpnType::Deposit => write!(f, "DEPOSIT"),
            IpnType::Withdrawal => write!(f, "WITHDRAWAL"),
        }
    }
}

/// Address type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AddressType {
    Temporary,
    Permanent,
}

/// Parameters for creating a deposit
#[derive(Debug, Clone, Serialize)]
pub struct CreateDepositParams {
    #[serde(rename = "networkType")]
    pub network_type: NetworkType,
    #[serde(rename = "contractAddress")]
    pub contract_address: String,
    pub amount: String,
    #[serde(rename = "ipnUrl")]
    pub ipn_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udf: Option<String>,
    /// Address lifetime, defaults to [`AddressType::Temporary`] server side
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub address_type: Option<AddressType>,
    /// Sweep the received funds to the merchant wallet, defaults to `true` server side
    #[serde(rename = "transferBalance", skip_serializing_if = "Option::is_none")]
    pub transfer_balance: Option<bool>,
}

/// Parameters for creating a withdrawal
#[derive(Debug, Clone, Serialize)]
pub struct CreateWithdrawalParams {
    #[serde(rename = "networkType")]
    pub network_type: NetworkType,
    #[serde(rename = "contractAddress")]
    pub contract_address: String,
    pub address: String,
    pub amount: String,
    #[serde(rename = "ipnUrl")]
    pub ipn_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udf: Option<String>,
}

/// Response from the create deposit endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct DepositResponse {
    pub id: String,
    pub address: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<DateTime<Utc>>,
    pub amount: String,
    pub status: TransactionStatus,
}

/// Response from the create withdrawal endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct WithdrawalResponse {
    pub id: String,
    pub status: TransactionStatus,
    pub fee: String,
}

/// Token information
///
/// `/check-transaction-status` echoes the network and contract of the token,
/// while `/fetch-wallet-balance` returns the token metadata only — hence the
/// two optional fields.
#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    #[serde(rename = "networkType", default)]
    pub network_type: Option<NetworkType>,
    #[serde(rename = "contractAddress", default)]
    pub contract_address: Option<String>,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

/// Transaction details
#[derive(Debug, Clone, Deserialize)]
pub struct Transaction {
    pub id: String,
    #[serde(rename = "transactionType")]
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
    /// Deposits only: number of block confirmations seen so far
    #[serde(default)]
    pub confirmations: Option<u64>,
    /// Withdrawals only: network fee charged for the transfer
    #[serde(default)]
    pub fee: Option<String>,
    /// Withdrawals only: when the transfer was broadcast
    #[serde(rename = "processedAt", default)]
    pub processed_at: Option<DateTime<Utc>>,
    #[serde(rename = "networkType")]
    pub network_type: NetworkType,
    #[serde(rename = "contractAddress")]
    pub contract_address: String,
    pub address: String,
    pub udf: Option<String>,
    pub token: Token,
}

/// Response from the transaction status endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionStatusResponse {
    pub address: Option<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: TransactionType,
    pub count: usize,
    pub transactions: Vec<Transaction>,
}

/// Balance information
#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    pub raw: String,
    pub formatted: String,
}

/// Response from the wallet balance endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct WalletBalanceResponse {
    pub address: String,
    #[serde(rename = "networkType")]
    pub network_type: NetworkType,
    #[serde(rename = "contractAddress")]
    pub contract_address: String,
    pub token: Token,
    pub balance: Balance,
}

/// Parameters for confirming an IPN with the SagaPay API
///
/// The API key and secret are taken from the client [`Config`](crate::Config)
/// and sent in the request body — `/verify-ipn` takes no auth headers.
#[derive(Debug, Clone, Serialize)]
pub struct VerifyIpnParams {
    #[serde(rename = "txnHash")]
    pub txn_hash: String,
    #[serde(rename = "type")]
    pub ipn_type: IpnType,
    pub amount: String,
    pub address: String,
}

/// Response from the verify IPN endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct VerifyIpnResponse {
    pub verified: bool,
}

/// Webhook payload
///
/// Note that `type` is uppercase here (`DEPOSIT` / `WITHDRAWAL`), unlike the
/// lowercase `type` query parameter of `/check-transaction-status`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebhookPayload {
    pub id: String,
    #[serde(rename = "type")]
    pub transaction_type: IpnType,
    pub status: TransactionStatus,
    pub address: String,
    #[serde(rename = "networkType")]
    pub network_type: NetworkType,
    pub amount: String,
    pub udf: Option<String>,
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_wallet_balance_with_metadata_only_token() {
        // `/fetch-wallet-balance` returns token metadata without the network
        // or contract, which live on the response itself.
        let body = r#"{
            "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            "networkType": "ERC20",
            "contractAddress": "0xdac17f958d2ee523a2206206994597c13d831ec7",
            "token": { "symbol": "USDT", "name": "Tether USD", "decimals": 6 },
            "balance": { "raw": "1500000", "formatted": "1.500000" }
        }"#;

        let response: WalletBalanceResponse = serde_json::from_str(body).unwrap();

        assert_eq!(response.network_type, NetworkType::ERC20);
        assert_eq!(
            response.contract_address,
            "0xdac17f958d2ee523a2206206994597c13d831ec7"
        );
        assert_eq!(response.token.symbol, "USDT");
        assert_eq!(response.token.decimals, 6);
        assert!(response.token.network_type.is_none());
        assert!(response.token.contract_address.is_none());
        assert_eq!(response.balance.formatted, "1.500000");
    }

    #[test]
    fn deserializes_native_wallet_balance() {
        // Native coins report "0" as the contract address.
        let body = r#"{
            "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            "networkType": "BEP20",
            "contractAddress": "0",
            "token": { "symbol": "BEP20", "name": "BEP20", "decimals": 18 },
            "balance": { "raw": "0", "formatted": "0.000000000000000000" }
        }"#;

        let response: WalletBalanceResponse = serde_json::from_str(body).unwrap();

        assert_eq!(response.contract_address, "0");
        assert_eq!(response.balance.raw, "0");
    }

    #[test]
    fn deserializes_deposit_transaction_status() {
        let body = r#"{
            "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            "transactionType": "deposit",
            "count": 1,
            "transactions": [
                {
                    "id": "8b7c1f2e-0f1a-4a4c-9c6b-1f0c7d5a2b3e",
                    "transactionType": "deposit",
                    "status": "COMPLETED",
                    "amount": "1.5",
                    "createdAt": "2025-03-16T14:30:00.000Z",
                    "updatedAt": "2025-03-16T14:35:00.000Z",
                    "txHash": "0xabc123",
                    "confirmations": 12,
                    "networkType": "BEP20",
                    "contractAddress": "0",
                    "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
                    "udf": "order-123",
                    "token": {
                        "networkType": "BEP20",
                        "contractAddress": "0",
                        "symbol": "BNB",
                        "name": "BNB",
                        "decimals": 18
                    }
                }
            ]
        }"#;

        let response: TransactionStatusResponse = serde_json::from_str(body).unwrap();

        assert_eq!(response.transaction_type, TransactionType::Deposit);
        assert_eq!(response.count, 1);

        let tx = &response.transactions[0];
        assert_eq!(tx.status, TransactionStatus::Completed);
        assert_eq!(tx.confirmations, Some(12));
        assert_eq!(tx.token.network_type, Some(NetworkType::BEP20));
        assert_eq!(tx.token.contract_address.as_deref(), Some("0"));
        // Deposits carry no fee or processing timestamp.
        assert!(tx.fee.is_none());
        assert!(tx.processed_at.is_none());
    }

    #[test]
    fn deserializes_withdrawal_transaction_status() {
        let body = r#"{
            "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            "transactionType": "withdrawal",
            "count": 1,
            "transactions": [
                {
                    "id": "0c1d2e3f-4a5b-6c7d-8e9f-0a1b2c3d4e5f",
                    "transactionType": "withdrawal",
                    "status": "PROCESSING",
                    "amount": "10.5",
                    "fee": "0.0005",
                    "createdAt": "2025-03-16T14:30:00.000Z",
                    "updatedAt": "2025-03-16T14:31:00.000Z",
                    "txHash": null,
                    "processedAt": null,
                    "networkType": "ERC20",
                    "contractAddress": "0xdac17f958d2ee523a2206206994597c13d831ec7",
                    "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
                    "udf": null,
                    "token": {
                        "networkType": "ERC20",
                        "contractAddress": "0xdac17f958d2ee523a2206206994597c13d831ec7",
                        "symbol": "USDT",
                        "name": "Tether USD",
                        "decimals": 6
                    }
                }
            ]
        }"#;

        let response: TransactionStatusResponse = serde_json::from_str(body).unwrap();
        let tx = &response.transactions[0];

        assert_eq!(response.transaction_type, TransactionType::Withdrawal);
        assert_eq!(tx.fee.as_deref(), Some("0.0005"));
        assert!(tx.processed_at.is_none());
        assert!(tx.tx_hash.is_none());
        assert!(tx.udf.is_none());
        // Withdrawals report no confirmation count.
        assert!(tx.confirmations.is_none());
    }

    #[test]
    fn deserializes_uppercase_ipn_payload() {
        let body = r#"{
            "id": "8b7c1f2e-0f1a-4a4c-9c6b-1f0c7d5a2b3e",
            "type": "DEPOSIT",
            "status": "COMPLETED",
            "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            "networkType": "BEP20",
            "amount": "1.5",
            "udf": null,
            "txHash": null,
            "timestamp": "2025-03-16T14:30:00.000Z"
        }"#;

        let payload: WebhookPayload = serde_json::from_str(body).unwrap();

        assert_eq!(payload.transaction_type, IpnType::Deposit);
        assert_eq!(payload.status, TransactionStatus::Completed);
        assert_eq!(payload.network_type, NetworkType::BEP20);
        assert!(payload.udf.is_none());
        assert!(payload.tx_hash.is_none());

        // Round-trips back to the uppercase wire shape.
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["type"], "DEPOSIT");
    }

    #[test]
    fn serializes_create_deposit_params() {
        let params = CreateDepositParams {
            network_type: NetworkType::BEP20,
            contract_address: "0".to_string(),
            amount: "1.5".to_string(),
            ipn_url: "https://example.com/webhook".to_string(),
            udf: None,
            address_type: Some(AddressType::Temporary),
            transfer_balance: Some(false),
        };

        let json = serde_json::to_value(&params).unwrap();

        assert_eq!(json["networkType"], "BEP20");
        assert_eq!(json["type"], "TEMPORARY");
        assert_eq!(json["transferBalance"], false);
        // Optional fields are omitted rather than sent as null.
        assert!(json.get("udf").is_none());
    }

    #[test]
    fn serializes_verify_ipn_params_with_uppercase_type() {
        let params = VerifyIpnParams {
            txn_hash: "0xabc123".to_string(),
            ipn_type: IpnType::Withdrawal,
            amount: "10.5".to_string(),
            address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
        };

        let json = serde_json::to_value(&params).unwrap();

        assert_eq!(json["txnHash"], "0xabc123");
        assert_eq!(json["type"], "WITHDRAWAL");
    }
}
