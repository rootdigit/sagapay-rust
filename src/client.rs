use reqwest::Client as ReqwestClient;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

use crate::config::Config;
use crate::error::Error;
use crate::models::{
    CreateDepositParams, CreateWithdrawalParams, DepositResponse, IpnType, NetworkType,
    TransactionStatusResponse, TransactionType, VerifyIpnParams, VerifyIpnResponse,
    WalletBalanceResponse, WithdrawalResponse,
};

/// SagaPay API client
#[derive(Debug, Clone)]
pub struct Client {
    config: Arc<Config>,
    http_client: ReqwestClient,
}

impl Client {
    /// Create a new SagaPay client
    pub fn new(config: Config) -> Self {
        let http_client = ReqwestClient::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config: Arc::new(config),
            http_client,
        }
    }

    /// Create a deposit address
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sagapay::{AddressType, Client, Config, CreateDepositParams, NetworkType};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), sagapay::Error> {
    ///     let client = Client::new(Config {
    ///         api_key: "your-api-key".to_string(),
    ///         api_secret: "your-api-secret".to_string(),
    ///         ..Config::default()
    ///     });
    ///
    ///     let deposit = client.create_deposit(CreateDepositParams {
    ///         network_type: NetworkType::BEP20,
    ///         contract_address: "0".to_string(),
    ///         amount: "1.5".to_string(),
    ///         ipn_url: "https://example.com/webhook".to_string(),
    ///         udf: Some("order-123".to_string()),
    ///         address_type: Some(AddressType::Temporary),
    ///         transfer_balance: None,
    ///     }).await?;
    ///
    ///     println!("Deposit address: {}", deposit.address);
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_deposit(
        &self,
        params: CreateDepositParams,
    ) -> Result<DepositResponse, Error> {
        self.post("/create-deposit", &params).await
    }

    /// Create a withdrawal
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sagapay::{Client, Config, CreateWithdrawalParams, NetworkType};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), sagapay::Error> {
    ///     let client = Client::new(Config {
    ///         api_key: "your-api-key".to_string(),
    ///         api_secret: "your-api-secret".to_string(),
    ///         ..Config::default()
    ///     });
    ///
    ///     let withdrawal = client.create_withdrawal(CreateWithdrawalParams {
    ///         network_type: NetworkType::ERC20,
    ///         contract_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT
    ///         address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
    ///         amount: "10.5".to_string(),
    ///         ipn_url: "https://example.com/webhook".to_string(),
    ///         udf: Some("withdrawal-456".to_string()),
    ///     }).await?;
    ///
    ///     println!("Withdrawal ID: {}", withdrawal.id);
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_withdrawal(
        &self,
        params: CreateWithdrawalParams,
    ) -> Result<WithdrawalResponse, Error> {
        self.post("/create-withdrawal", &params).await
    }

    /// Check transaction status by address or ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sagapay::{Client, Config, TransactionType};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), sagapay::Error> {
    ///     let client = Client::new(Config {
    ///         api_key: "your-api-key".to_string(),
    ///         api_secret: "your-api-secret".to_string(),
    ///         ..Config::default()
    ///     });
    ///
    ///     // By address
    ///     let status = client.check_transaction_status(
    ///         TransactionType::Deposit,
    ///         Some("0x742d35Cc6634C0532925a3b844Bc454e4438f44e"),
    ///         None,
    ///     ).await?;
    ///
    ///     // By ID
    ///     let status = client.check_transaction_status(
    ///         TransactionType::Deposit,
    ///         None,
    ///         Some("deposit-uuid"),
    ///     ).await?;
    ///
    ///     println!("Transaction count: {}", status.count);
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_transaction_status(
        &self,
        transaction_type: TransactionType,
        address: Option<&str>,
        id: Option<&str>,
    ) -> Result<TransactionStatusResponse, Error> {
        if address.is_none() && id.is_none() {
            return Err(Error::MissingParameter("address or id".to_string()));
        }

        let tx_type = match transaction_type {
            TransactionType::Deposit => "deposit",
            TransactionType::Withdrawal => "withdrawal",
        };

        let mut params: Vec<(&str, &str)> = vec![("type", tx_type)];
        if let Some(addr) = address {
            params.push(("address", addr));
        }
        if let Some(txn_id) = id {
            params.push(("id", txn_id));
        }

        self.get_with_query("/check-transaction-status", &params)
            .await
    }

    /// Fetch wallet balance
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sagapay::{Client, Config, NetworkType};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), sagapay::Error> {
    ///     let client = Client::new(Config {
    ///         api_key: "your-api-key".to_string(),
    ///         api_secret: "your-api-secret".to_string(),
    ///         ..Config::default()
    ///     });
    ///
    ///     let address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    ///     let network_type = NetworkType::ERC20;
    ///     let contract_address = Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"); // USDT
    ///
    ///     let balance = client.fetch_wallet_balance(address, network_type, contract_address).await?;
    ///
    ///     println!("Token: {} ({})", balance.token.symbol, balance.token.name);
    ///     println!("Balance: {}", balance.balance.formatted);
    ///
    ///     // Pass `None` (or `Some("0")`) for the network's native coin
    ///     let native = client
    ///         .fetch_wallet_balance(address, NetworkType::ERC20, None)
    ///         .await?;
    ///
    ///     println!("Native balance: {}", native.balance.formatted);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_wallet_balance(
        &self,
        address: &str,
        network_type: NetworkType,
        contract_address: Option<&str>,
    ) -> Result<WalletBalanceResponse, Error> {
        let network_type = network_type.to_string();

        let mut params: Vec<(&str, &str)> =
            vec![("address", address), ("networkType", &network_type)];
        if let Some(contract) = contract_address {
            params.push(("contractAddress", contract));
        }

        self.get_with_query("/fetch-wallet-balance", &params).await
    }

    /// Confirm an IPN with the SagaPay API
    ///
    /// This is the primary way to validate a notification: the API re-checks
    /// the transaction against its own records and answers with `verified`.
    /// Unlike every other endpoint, `/verify-ipn` takes no auth headers — the
    /// API key and secret are read from the client [`Config`] and sent in the
    /// request body.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sagapay::{Client, Config, IpnType, VerifyIpnParams};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), sagapay::Error> {
    ///     let client = Client::new(Config {
    ///         api_key: "your-api-key".to_string(),
    ///         api_secret: "your-api-secret".to_string(),
    ///         ..Config::default()
    ///     });
    ///
    ///     let result = client.verify_ipn(VerifyIpnParams {
    ///         txn_hash: "0xabc123".to_string(),
    ///         ipn_type: IpnType::Deposit,
    ///         amount: "1.5".to_string(),
    ///         address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
    ///     }).await?;
    ///
    ///     println!("Verified: {}", result.verified);
    ///     Ok(())
    /// }
    /// ```
    pub async fn verify_ipn(&self, params: VerifyIpnParams) -> Result<VerifyIpnResponse, Error> {
        /// Request body for `/verify-ipn`, which carries the credentials inline
        #[derive(Serialize)]
        struct VerifyIpnRequest<'a> {
            #[serde(rename = "txnHash")]
            txn_hash: &'a str,
            #[serde(rename = "type")]
            ipn_type: &'a IpnType,
            amount: &'a str,
            address: &'a str,
            #[serde(rename = "apiKey")]
            api_key: &'a str,
            #[serde(rename = "apiSecret")]
            api_secret: &'a str,
        }

        let body = VerifyIpnRequest {
            txn_hash: &params.txn_hash,
            ipn_type: &params.ipn_type,
            amount: &params.amount,
            address: &params.address,
            api_key: &self.config.api_key,
            api_secret: &self.config.api_secret,
        };

        let url = format!("{}/verify-ipn", self.config.base_url);

        // Deliberately built without the x-api-key / x-api-secret headers
        let response = self
            .http_client
            .post(&url)
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .json(&body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get the client's API key
    pub fn api_key(&self) -> &str {
        &self.config.api_key
    }

    /// Get the client's API secret
    pub fn api_secret(&self) -> &str {
        &self.config.api_secret
    }

    /// Make a GET request with query parameters
    async fn get_with_query<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.config.base_url, path);
        let headers = self.build_headers()?;

        let response = self
            .http_client
            .get(&url)
            .headers(headers)
            .query(query)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Make a POST request
    async fn post<T: DeserializeOwned, D: Serialize + ?Sized>(
        &self,
        path: &str,
        data: &D,
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.config.base_url, path);
        let headers = self.build_headers()?;

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(data)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Build common headers for requests
    fn build_headers(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();

        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|e| Error::InvalidParameter(format!("api_key: {}", e)))?,
        );
        headers.insert(
            "x-api-secret",
            HeaderValue::from_str(&self.config.api_secret)
                .map_err(|e| Error::InvalidParameter(format!("api_secret: {}", e)))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        Ok(headers)
    }

    /// Handle API response
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, Error> {
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            let error_response: serde_json::Value = match serde_json::from_str(&body) {
                Ok(json) => json,
                Err(_) => {
                    return Err(Error::ApiError {
                        status_code: status.as_u16(),
                        message: body,
                        error_code: None,
                    });
                }
            };

            // Error bodies are shaped { "error": "<human readable message>" }
            let message = error_response
                .get("error")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();

            return Err(Error::ApiError {
                status_code: status.as_u16(),
                message,
                error_code: None,
            });
        }

        match serde_json::from_str(&body) {
            Ok(data) => Ok(data),
            Err(e) => Err(Error::JsonError(e)),
        }
    }
}
