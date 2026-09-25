/// Configuration for the SagaPay client
#[derive(Debug, Clone)]
pub struct Config {
    /// SagaPay API key
    pub api_key: String,

    /// SagaPay API secret
    pub api_secret: String,

    /// API base URL
    pub base_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_secret: String::new(),
            base_url: "https://api2.sagapay.io".to_string(),
        }
    }
}
