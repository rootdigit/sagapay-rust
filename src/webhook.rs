use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::collections::HashMap;

use crate::error::Error;
use crate::models::WebhookPayload;

/// Header carrying the IPN signature, formatted as `sha256=<hex>`
const SIGNATURE_HEADER: &str = "X-Sagapay-Signature";

/// Handler for SagaPay webhook notifications (IPN)
///
/// IPNs are delivered at least once, so treat the payload `id` as an
/// idempotency key and make your handler safe to run twice.
#[derive(Debug, Clone)]
pub struct WebhookHandler {
    ipn_secret: Option<String>,
}

impl WebhookHandler {
    /// Create a new webhook handler
    ///
    /// `ipn_secret` is the **optional platform-issued IPN secret** used to sign
    /// outbound notifications. It is *not* your API secret — SagaPay only ever
    /// stores a hash of the API secret, so it can never be the signing key.
    ///
    /// Pass `None` if you have not been issued an IPN secret. Signature
    /// verification is then skipped and the payload is only parsed, so confirm
    /// every notification with [`Client::verify_ipn`](crate::Client::verify_ipn)
    /// before acting on it.
    pub fn new(ipn_secret: Option<String>) -> Self {
        Self { ipn_secret }
    }

    /// Process and verify a webhook
    ///
    /// When an IPN secret is configured, `signature` must be present and match
    /// the body, otherwise [`Error::InvalidSignature`] is returned. Without a
    /// secret the signature is ignored and the body is only parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sagapay::{Error, WebhookHandler};
    /// use std::collections::HashMap;
    ///
    /// async fn handle_webhook(
    ///     headers: HashMap<String, String>,
    ///     body: &str,
    /// ) -> Result<(), Error> {
    ///     // Platform-issued IPN secret, not your API secret. `None` skips
    ///     // signature verification.
    ///     let handler = WebhookHandler::new(Some("your-ipn-secret".to_string()));
    ///
    ///     // Get the signature from headers, e.g. "sha256=9f86d0..."
    ///     let signature = headers.get("x-sagapay-signature").map(|s| s.as_str());
    ///
    ///     // Process and validate the webhook
    ///     let payload = handler.process_webhook(body, signature)?;
    ///
    ///     // Handle the webhook data
    ///     println!("Transaction ID: {}", payload.id);
    ///     println!("Type: {:?}", payload.transaction_type);
    ///     println!("Status: {:?}", payload.status);
    ///     println!("Amount: {}", payload.amount);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn process_webhook(
        &self,
        body: &str,
        signature: Option<&str>,
    ) -> Result<WebhookPayload, Error> {
        // Verify the signature, unless no IPN secret was configured
        if self.ipn_secret.is_some() {
            let signature = signature.ok_or(Error::InvalidSignature)?;

            if !self.verify_signature(body, signature) {
                return Err(Error::InvalidSignature);
            }
        }

        // Parse the webhook payload
        let payload: WebhookPayload = serde_json::from_str(body)?;

        Ok(payload)
    }

    /// Helper method to extract headers and process a webhook in a single step
    ///
    /// The header name is matched case-insensitively, so header maps that
    /// preserve the original casing work as well as normalised ones.
    pub fn process_webhook_from_request(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<WebhookPayload, Error> {
        // Get the signature from headers
        let signature = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(SIGNATURE_HEADER))
            .map(|(_, value)| value.as_str());

        self.process_webhook(body, signature)
    }

    /// Verify the HMAC signature of a webhook payload
    ///
    /// Accepts both the `sha256=<hex>` form sent by SagaPay and a bare hex
    /// digest, and compares in constant time.
    fn verify_signature(&self, payload: &str, signature: &str) -> bool {
        let Some(secret) = self.ipn_secret.as_deref() else {
            return false;
        };

        // The header is sent as "sha256=<hex>"; tolerate a bare digest too
        let signature = signature.strip_prefix("sha256=").unwrap_or(signature);

        let provided = match hex::decode(signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");

        mac.update(payload.as_bytes());

        // Constant-time comparison against the expected digest
        mac.verify_slice(&provided).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IpnType, TransactionStatus};

    const IPN_SECRET: &str = "platform-issued-ipn-secret";

    const BODY: &str = r#"{"id":"8b7c1f2e-0f1a-4a4c-9c6b-1f0c7d5a2b3e","type":"DEPOSIT","status":"COMPLETED","address":"0x742d35Cc6634C0532925a3b844Bc454e4438f44e","networkType":"BEP20","amount":"1.5","udf":"order-123","txHash":null,"timestamp":"2025-03-16T14:30:00.000Z"}"#;

    /// Sign a body the way the SagaPay IPN sender does
    fn sign(secret: &str, body: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(body.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    fn handler() -> WebhookHandler {
        WebhookHandler::new(Some(IPN_SECRET.to_string()))
    }

    #[test]
    fn accepts_prefixed_signature() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));

        let payload = handler().process_webhook(BODY, Some(&signature)).unwrap();

        assert_eq!(payload.transaction_type, IpnType::Deposit);
        assert_eq!(payload.status, TransactionStatus::Completed);
        assert_eq!(payload.udf.as_deref(), Some("order-123"));
    }

    #[test]
    fn accepts_bare_hex_signature() {
        let signature = sign(IPN_SECRET, BODY);

        let payload = handler().process_webhook(BODY, Some(&signature)).unwrap();

        assert_eq!(payload.id, "8b7c1f2e-0f1a-4a4c-9c6b-1f0c7d5a2b3e");
    }

    #[test]
    fn rejects_tampered_body() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));
        let tampered = BODY.replace("\"1.5\"", "\"9999\"");

        let result = handler().process_webhook(&tampered, Some(&signature));

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn rejects_signature_from_another_secret() {
        let signature = format!("sha256={}", sign("someone-elses-secret", BODY));

        let result = handler().process_webhook(BODY, Some(&signature));

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn rejects_invalid_hex() {
        let result = handler().process_webhook(BODY, Some("sha256=not-hex"));

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn rejects_missing_signature_when_secret_is_configured() {
        let result = handler().process_webhook(BODY, None);

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn skips_verification_without_a_secret() {
        let handler = WebhookHandler::new(None);

        // No signature at all, and a bogus one, are both accepted
        let payload = handler.process_webhook(BODY, None).unwrap();
        assert_eq!(payload.amount, "1.5");

        let payload = handler
            .process_webhook(BODY, Some("sha256=deadbeef"))
            .unwrap();
        assert_eq!(payload.amount, "1.5");
    }

    #[test]
    fn finds_the_signature_header_regardless_of_case() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-SAGAPAY-SIGNATURE".to_string(), signature);

        let payload = handler()
            .process_webhook_from_request(&headers, BODY)
            .unwrap();

        assert_eq!(payload.amount, "1.5");
    }

    #[test]
    fn rejects_a_request_without_the_signature_header() {
        let headers = HashMap::new();

        let result = handler().process_webhook_from_request(&headers, BODY);

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }
}
