use sagapay::{
    AddressType, Client, Config, CreateDepositParams, CreateWithdrawalParams, NetworkType,
    TransactionType,
};

#[tokio::main]
async fn main() -> Result<(), sagapay::Error> {
    // Initialize the SagaPay client
    let client = Client::new(Config {
        api_key: "your-api-key".to_string(),
        api_secret: "your-api-secret".to_string(),
        ..Config::default()
    });

    // Example 1: Create a deposit address
    println!("Creating deposit address...");
    let deposit = client
        .create_deposit(CreateDepositParams {
            network_type: NetworkType::BEP20,
            contract_address: "0".to_string(), // Use '0' for native tokens (BNB)
            amount: "1.5".to_string(),
            ipn_url: "https://yourwebsite.com/webhook".to_string(),
            udf: Some("order-123".to_string()),
            address_type: Some(AddressType::Temporary),
            transfer_balance: None, // Defaults to true: sweep funds to your wallet
        })
        .await?;

    println!("✓ Deposit address created:");
    println!("  ID: {}", deposit.id);
    println!("  Address: {}", deposit.address);
    println!("  Expires: {:?}", deposit.expires_at); // None for permanent addresses
    println!("  Status: {:?}", deposit.status);
    println!();

    // Example 2: Create a withdrawal
    println!("Creating withdrawal...");
    let withdrawal = client
        .create_withdrawal(CreateWithdrawalParams {
            network_type: NetworkType::ERC20,
            contract_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT on Ethereum
            address: "0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string(),
            amount: "10.5".to_string(),
            ipn_url: "https://yourwebsite.com/webhook".to_string(),
            udf: Some("withdrawal-456".to_string()),
        })
        .await?;

    println!("✓ Withdrawal created:");
    println!("  ID: {}", withdrawal.id);
    println!("  Status: {:?}", withdrawal.status);
    println!("  Fee: {}", withdrawal.fee);
    println!();

    // Example 3: Check transaction status
    println!("Checking transaction status...");
    let address = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
    let tx_status = client
        .check_transaction_status(TransactionType::Deposit, Some(address), None)
        .await?;

    println!("✓ Transaction status retrieved:");
    println!("  Address: {}", tx_status.address.as_deref().unwrap_or("-"));
    println!("  Type: {:?}", tx_status.transaction_type);
    println!("  Count: {}", tx_status.count);

    if tx_status.count > 0 {
        println!("  Transactions:");
        for (i, tx) in tx_status.transactions.iter().enumerate() {
            println!(
                "    #{} ID: {}, Status: {:?}, Amount: {}, Confirmations: {}",
                i + 1,
                tx.id,
                tx.status,
                tx.amount,
                tx.confirmations.unwrap_or(0)
            );
        }
    }
    println!();

    // Example 4: Fetch wallet balance
    println!("Fetching wallet balance...");
    let balance = client
        .fetch_wallet_balance(
            "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            NetworkType::ERC20,
            Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"), // USDT on Ethereum
        )
        .await?;

    println!("✓ Wallet balance retrieved:");
    println!("  Address: {}", balance.address);
    println!("  Token: {} ({})", balance.token.symbol, balance.token.name);
    println!("  Balance: {}", balance.balance.formatted);
    println!();

    // Example 5: Fetch a native coin balance by omitting the contract address
    println!("Fetching native balance...");
    let native_balance = client
        .fetch_wallet_balance(
            "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
            NetworkType::ERC20,
            None,
        )
        .await?;

    println!("✓ Native balance retrieved:");
    println!(
        "  {} {}",
        native_balance.balance.formatted, native_balance.token.symbol
    );

    Ok(())
}
