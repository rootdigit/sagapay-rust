# Changelog

## 0.3.0 — 2026-10-27

### Breaking
- `WebhookHandler::new` takes your signing secret, `impl Into<String>`, instead of `Option<String>`. Copy the secret from the dashboard (Webhooks → Signing secret). A handler can no longer skip signature verification: with an empty secret it rejects every notification.
- Signatures are read from `X-Sagapay-Merchant-Signature`, keyed with your own signing secret. The old `X-Sagapay-Signature` header, keyed with a secret shared by every merchant, is retired: SagaPay no longer sends it and this SDK no longer reads it.
- New `Error::StaleWebhook` variant.

### Security
- The HTTP client no longer follows redirects. reqwest resends custom headers such as `x-api-key` and `x-api-secret` on a redirect.
- `process_webhook_from_request` rejects a signature header present under more than one casing. Upper-case hex and `SHA256=` are accepted.
- `WebhookHandler`'s `Debug` output no longer shows the secret.

### Added
- `is_ipn_fresh(&payload, max_age, now)` and `WebhookHandler::with_max_age(max_age)` reject notifications whose `timestamp` is too old or more than 5 minutes in the future. Use them alongside processing each `id` only once.
- `SIGNATURE_HEADER` and `DEFAULT_MAX_AGE` constants; `WebhookHandler::verify_signature` is now public.
- `Client::create_withdrawal_idempotent(params, key)` sends an `Idempotency-Key` header, so a retried request cannot create a second withdrawal.
- `WebhookPayload` has the `asset` and `contract_address` fields that notifications carry.
