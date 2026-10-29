# Download grant issuer

When `WOO_DOWNLOAD_GRANT_SECRET` is set to a 32-byte hex secret, the game server serves plain HTTP on `127.0.0.1:8088`. Set `WOO_DOWNLOAD_GRANT_PORT` to change the port. An invalid secret or port, or a failed bind, stops server startup. Without the secret the issuer is disabled.

`POST /v1/download-grant` accepts JSON containing either `username` and `password`, or `token`. A successful login returns `200` JSON with `grant`, `expires_at` (Unix seconds) and an account `token` to save. Invalid credentials or a banned account return `401` JSON with `error`. The grant lasts six hours and has the form `v1.BASE64URL(payload).BASE64URL(mac)` without padding; payload is UTF-8 JSON `{"a":account_id,"e":expiry}` and the MAC is HMAC-SHA256 over the first two dot-separated fields using the decoded secret. Expiry at or before the current time is invalid.

The issuer runs in the server process and shares its open redb database: a separate process would conflict with redb's database lock, and password login must persist a token. HTTP is loopback only; the external HTTPS tunnel is configured separately.
