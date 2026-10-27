use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::time::Duration;

use crate::error::Error;
use crate::models::WebhookPayload;

/// Header carrying the IPN signature, formatted as `sha256=<hex>`
pub const SIGNATURE_HEADER: &str = "X-Sagapay-Merchant-Signature";

/// A sensible limit for [`is_ipn_fresh`] and [`WebhookHandler::with_max_age`]:
/// 24 hours, well above the IPN retry schedule (about 7.5 hours).
pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// How far ahead of this machine's clock a timestamp may be before it is
/// treated as invalid rather than as clock drift.
const MAX_CLOCK_SKEW: chrono::Duration = chrono::Duration::minutes(5);

/// Whether a notification's timestamp is recent enough to act on, as a guard
/// against an old, validly signed notification being replayed.
///
/// The timestamp is set when the transaction completes and stays the same on
/// every retry, so keep `max_age` longer than the retry schedule (about 7.5
/// hours). A notification resent from the dashboard also keeps its original
/// timestamp, and is rejected once it is older than `max_age`.
///
/// This is a secondary check. The main defence against replays is to process
/// each notification `id` only once.
pub fn is_ipn_fresh(payload: &WebhookPayload, max_age: Duration, now: DateTime<Utc>) -> bool {
    let Ok(max_age) = chrono::Duration::from_std(max_age) else {
        return true; // longer than chrono can represent: no limit
    };
    payload.timestamp <= now + MAX_CLOCK_SKEW && now - payload.timestamp <= max_age
}

/// Handler for SagaPay webhook notifications (IPN)
///
/// Every notification is checked against its `X-Sagapay-Merchant-Signature`
/// header. IPNs are delivered at least once, and a replayed request carries a
/// valid signature too, so treat the payload `id` as an idempotency key and
/// make your handler safe to run twice.
#[derive(Clone)]
pub struct WebhookHandler {
    signing_secret: String,
    max_age: Option<Duration>,
}

impl std::fmt::Debug for WebhookHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookHandler")
            .field("signing_secret", &"<redacted>")
            .field("max_age", &self.max_age)
            .finish()
    }
}

impl WebhookHandler {
    /// Create a new webhook handler
    ///
    /// `signing_secret` is your signing secret: copy it from the dashboard
    /// (Webhooks → Signing secret). It is *not* your API secret. A handler
    /// with an empty secret rejects every notification.
    pub fn new(signing_secret: impl Into<String>) -> Self {
        Self {
            signing_secret: signing_secret.into(),
            max_age: None,
        }
    }

    /// Also reject notifications whose timestamp is older than `max_age`
    /// (see [`is_ipn_fresh`]). Keep it above the ~7.5 hour retry schedule;
    /// [`DEFAULT_MAX_AGE`] is a sensible value.
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }

    /// Verify and parse a webhook
    ///
    /// `body` must be the exact raw request body and `signature` the value of
    /// the `X-Sagapay-Merchant-Signature` header. A missing or wrong signature
    /// returns [`Error::InvalidSignature`]; with [`with_max_age`](Self::with_max_age),
    /// a stale notification returns [`Error::StaleWebhook`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sagapay::{Error, WebhookHandler, SIGNATURE_HEADER};
    /// use std::collections::HashMap;
    ///
    /// async fn handle_webhook(
    ///     headers: HashMap<String, String>,
    ///     body: &str,
    /// ) -> Result<(), Error> {
    ///     // Dashboard → Webhooks → Signing secret, not your API secret
    ///     let handler = WebhookHandler::new(std::env::var("SAGAPAY_SIGNING_SECRET").unwrap_or_default());
    ///
    ///     // Or use process_webhook_from_request(&headers, body)
    ///     let signature = headers.get(&SIGNATURE_HEADER.to_ascii_lowercase()).map(|s| s.as_str());
    ///     let payload = handler.process_webhook(body, signature)?;
    ///
    ///     // Act on each id only once
    ///     println!("Transaction ID: {}", payload.id);
    ///     println!("Type: {:?}", payload.transaction_type);
    ///     println!("Asset: {:?}", payload.asset);
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
        let signature = signature.ok_or(Error::InvalidSignature)?;
        if !self.verify_signature(body, signature) {
            return Err(Error::InvalidSignature);
        }

        let payload: WebhookPayload = serde_json::from_str(body)?;

        if let Some(max_age) = self.max_age {
            if !is_ipn_fresh(&payload, max_age, Utc::now()) {
                return Err(Error::StaleWebhook);
            }
        }

        Ok(payload)
    }

    /// Helper method to extract headers and process a webhook in a single step
    ///
    /// The header name is matched case-insensitively, so header maps that
    /// preserve the original casing work as well as normalised ones. A header
    /// present under more than one casing is rejected.
    pub fn process_webhook_from_request(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> Result<WebhookPayload, Error> {
        let mut matches = headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case(SIGNATURE_HEADER))
            .map(|(_, value)| value.as_str());
        let signature = matches.next();
        if matches.next().is_some() {
            return Err(Error::InvalidSignature);
        }

        self.process_webhook(body, signature)
    }

    /// Verify the HMAC signature of a webhook payload
    ///
    /// Accepts both the `sha256=<hex>` form sent by SagaPay and a bare hex
    /// digest, and compares in constant time. False when the handler has no
    /// signing secret.
    pub fn verify_signature(&self, payload: &str, signature: &str) -> bool {
        if self.signing_secret.is_empty() {
            return false;
        }

        // The header is sent as "sha256=<hex>"; tolerate a bare digest too
        let signature = signature.trim();
        let signature = match signature.get(..7) {
            Some(prefix) if prefix.eq_ignore_ascii_case("sha256=") => &signature[7..],
            _ => signature,
        };

        let provided = match hex::decode(signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let mut mac = Hmac::<Sha256>::new_from_slice(self.signing_secret.as_bytes())
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

    /// A merchant signing secret is a 64-character hex string, used as the
    /// HMAC key exactly as written
    const IPN_SECRET: &str = "a3f1c2e4b5d697881234567890abcdef00112233445566778899aabbccddeeff";

    const BODY: &str = r#"{"id":"8b7c1f2e-0f1a-4a4c-9c6b-1f0c7d5a2b3e","type":"DEPOSIT","status":"COMPLETED","address":"0x742d35Cc6634C0532925a3b844Bc454e4438f44e","networkType":"BEP20","asset":"USDT","contractAddress":"0x55d398326f99059fF775485246999027B3197955","amount":"1.5","udf":"order-123","txHash":null,"timestamp":"2025-03-16T14:30:00.000Z"}"#;

    /// Sign a body the way the SagaPay IPN sender does
    fn sign(secret: &str, body: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(body.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    fn handler() -> WebhookHandler {
        WebhookHandler::new(IPN_SECRET)
    }

    #[test]
    fn accepts_prefixed_signature() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));

        let payload = handler().process_webhook(BODY, Some(&signature)).unwrap();

        assert_eq!(payload.transaction_type, IpnType::Deposit);
        assert_eq!(payload.status, TransactionStatus::Completed);
        assert_eq!(payload.udf.as_deref(), Some("order-123"));
        assert_eq!(payload.asset.as_deref(), Some("USDT"));
        assert_eq!(
            payload.contract_address.as_deref(),
            Some("0x55d398326f99059fF775485246999027B3197955")
        );
    }

    #[test]
    fn accepts_bare_and_upper_case_hex() {
        let digest = sign(IPN_SECRET, BODY);

        assert!(handler().process_webhook(BODY, Some(&digest)).is_ok());
        let upper = format!("SHA256={}", digest.to_ascii_uppercase());
        assert!(handler().process_webhook(BODY, Some(&upper)).is_ok());
    }

    #[test]
    fn matches_the_sagapay_signer() {
        // Produced by the SagaPay signer (core-api utils/ipnSignature.js) for
        // a test merchant; keeps this SDK in step with what the platform sends
        let handler =
            WebhookHandler::new("f59949bf79a73e05a41f10d2333a41b80644430f5220f468171392df355663a7");
        let body = r#"{"id":"dep-1","type":"DEPOSIT","status":"COMPLETED","address":"0xabc","networkType":"BEP20","asset":"USDT","contractAddress":"0x55d398326f99059fF775485246999027B3197955","amount":"100.5","udf":null,"txHash":"0xdef","timestamp":"2026-10-27T00:00:00.000Z"}"#;
        let header = "sha256=a9841956a4b8ccf048f953081db6892e69da181b5792dfcf8604ea4795f6a469";

        assert!(handler.process_webhook(body, Some(header)).is_ok());
        assert!(!handler.verify_signature(&body.replace("100.5", "100.6"), header));
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
    fn rejects_invalid_and_truncated_hex() {
        let digest = sign(IPN_SECRET, BODY);
        for bad in [
            "sha256=not-hex",
            "sha256=",
            &digest[..62],
            &format!("{digest}00"),
        ] {
            let result = handler().process_webhook(BODY, Some(bad));
            assert!(matches!(result, Err(Error::InvalidSignature)), "{bad}");
        }
    }

    #[test]
    fn rejects_missing_signature() {
        let result = handler().process_webhook(BODY, None);

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn rejects_everything_without_a_secret() {
        let handler = WebhookHandler::new("");
        let signature = format!("sha256={}", sign("", BODY));

        let result = handler.process_webhook(BODY, Some(&signature));

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn debug_output_hides_the_secret() {
        assert!(!format!("{:?}", handler()).contains(IPN_SECRET));
    }

    #[test]
    fn finds_the_signature_header_regardless_of_case() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-SAGAPAY-MERCHANT-SIGNATURE".to_string(), signature);

        let payload = handler()
            .process_webhook_from_request(&headers, BODY)
            .unwrap();

        assert_eq!(payload.amount, "1.5");
    }

    #[test]
    fn ignores_the_retired_platform_header() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));
        let mut headers = HashMap::new();
        headers.insert("X-Sagapay-Signature".to_string(), signature);

        let result = handler().process_webhook_from_request(&headers, BODY);

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn rejects_a_header_repeated_under_two_casings() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));
        let mut headers = HashMap::new();
        headers.insert(
            "x-sagapay-merchant-signature".to_string(),
            signature.clone(),
        );
        headers.insert("X-Sagapay-Merchant-Signature".to_string(), signature);

        let result = handler().process_webhook_from_request(&headers, BODY);

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn rejects_a_request_without_the_signature_header() {
        let headers = HashMap::new();

        let result = handler().process_webhook_from_request(&headers, BODY);

        assert!(matches!(result, Err(Error::InvalidSignature)));
    }

    #[test]
    fn freshness_window() {
        let payload: WebhookPayload = serde_json::from_str(BODY).unwrap();
        let sent = payload.timestamp;
        let hour = Duration::from_secs(3600);

        assert!(is_ipn_fresh(
            &payload,
            hour,
            sent + chrono::Duration::minutes(1)
        ));
        assert!(!is_ipn_fresh(
            &payload,
            hour,
            sent + chrono::Duration::seconds(3601)
        ));
        assert!(is_ipn_fresh(
            &payload,
            DEFAULT_MAX_AGE,
            sent + chrono::Duration::hours(8)
        ));
        assert!(!is_ipn_fresh(
            &payload,
            DEFAULT_MAX_AGE,
            sent + chrono::Duration::hours(25)
        ));
        assert!(is_ipn_fresh(
            &payload,
            hour,
            sent - chrono::Duration::minutes(1)
        ));
        assert!(!is_ipn_fresh(
            &payload,
            hour,
            sent - chrono::Duration::minutes(10)
        ));
    }

    #[test]
    fn with_max_age_rejects_a_stale_webhook() {
        let signature = format!("sha256={}", sign(IPN_SECRET, BODY));

        // BODY is from 2025, long past
        let result = handler()
            .with_max_age(DEFAULT_MAX_AGE)
            .process_webhook(BODY, Some(&signature));
        assert!(matches!(result, Err(Error::StaleWebhook)));

        assert!(handler().process_webhook(BODY, Some(&signature)).is_ok());
    }
}
