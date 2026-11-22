//! Authentication client — mirrors `/api/auth/*`.
//!
//! On successful login/refresh/OTP/anonymous flows the session token is
//! stored on the shared [`HttpClient`], so every subsequent request is
//! authenticated automatically.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::Result;
use crate::http::{HttpClient, json_body, json_of};
use crate::types::{LoginResponse, LoginResult, User};

/// Email + password credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

/// Options for `PATCH /api/auth/me`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateMeOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Map<String, Value>>,
    /// New password (also used to link anonymous accounts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Required when the account already has a password.
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "current_password",
        alias = "currentPassword"
    )]
    pub current_password: Option<String>,
    /// New email — links the account when set on an anonymous user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// TOTP enrollment response: secret + otpauth URI to scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSetup {
    pub secret: String,
    #[serde(rename = "otpauth_url")]
    pub otpauth_url: String,
}

/// Authentication client.
#[derive(Debug, Clone)]
pub struct AuthClient {
    http: HttpClient,
}

impl AuthClient {
    pub(crate) fn new(http: HttpClient) -> Self {
        AuthClient { http }
    }

    /// Creates the account and returns the user — call [`login`](Self::login) next.
    pub async fn register(&self, creds: &Credentials) -> Result<User> {
        self.http.post("/api/auth/register", json_of(creds)?).await
    }

    ///
    /// Email + password login. When the account has MFA enabled the result is
    /// [`LoginResult::MfaChallenge`] — finish with [`mfa_login`](Self::mfa_login).
    /// On success the session token is stored automatically.
    pub async fn login(&self, creds: &Credentials) -> Result<LoginResult> {
        let res: LoginResult = self.http.post("/api/auth/login", json_of(creds)?).await?;
        if let Some(token) = res.token() {
            self.http.set_token(Some(token.to_string()));
        }
        Ok(res)
    }

    /// Password + TOTP code — completes an MFA login challenge.
    pub async fn mfa_login(
        &self,
        email: &str,
        password: &str,
        code: &str,
    ) -> Result<LoginResponse> {
        let body = json!({ "email": email, "password": password, "code": code });
        let res: LoginResponse = self
            .http
            .post("/api/auth/mfa/login", json_body(body))
            .await?;
        self.http.set_token(Some(res.token.clone()));
        Ok(res)
    }

    /// Exchange the current token for a fresh one (PocketBase `auth-refresh`).
    pub async fn refresh(&self) -> Result<LoginResponse> {
        let res: LoginResponse = self.http.post("/api/auth/refresh", None).await?;
        self.http.set_token(Some(res.token.clone()));
        Ok(res)
    }

    /// Clears the in-memory session token.
    pub fn logout(&self) {
        self.http.set_token(None);
    }

    /// `GET /api/auth/me` — the authenticated user.
    pub async fn me(&self) -> Result<User> {
        self.http.get("/api/auth/me").await
    }

    /// `PATCH /api/auth/me` — update custom `data`, password, or email.
    pub async fn update_me(&self, opts: &UpdateMeOptions) -> Result<User> {
        self.http.patch("/api/auth/me", json_of(opts)?).await
    }

    /// Sends a password-reset email; response may include `dev_token` w/o SMTP.
    pub async fn forgot_password(&self, email: &str) -> Result<Value> {
        self.http
            .post(
                "/api/auth/forgot-password",
                json_body(json!({ "email": email })),
            )
            .await
    }

    /// `POST /api/auth/reset-password`.
    pub async fn reset_password(&self, token: &str, password: &str) -> Result<Value> {
        self.http
            .post(
                "/api/auth/reset-password",
                json_body(json!({ "token": token, "password": password })),
            )
            .await
    }

    /// Sends a verification email; response may include `dev_token` w/o SMTP.
    pub async fn request_verification(&self, email: &str) -> Result<Value> {
        self.http
            .post(
                "/api/auth/verify-email",
                json_body(json!({ "email": email })),
            )
            .await
    }

    /// `POST /api/auth/verify-email/confirm`.
    pub async fn confirm_verification(&self, token: &str) -> Result<Value> {
        self.http
            .post(
                "/api/auth/verify-email/confirm",
                json_body(json!({ "token": token })),
            )
            .await
    }

    /// Sends a 6-digit OTP; response may include `dev_otp` w/o SMTP.
    pub async fn request_otp(&self, email: &str) -> Result<Value> {
        self.http
            .post(
                "/api/auth/request-otp",
                json_body(json!({ "email": email })),
            )
            .await
    }

    /// OTP login — stores the session token on success.
    pub async fn auth_with_otp(&self, email: &str, otp: &str) -> Result<LoginResponse> {
        let body = json!({ "email": email, "otp": otp });
        let res: LoginResponse = self
            .http
            .post("/api/auth/auth-with-otp", json_body(body))
            .await?;
        self.http.set_token(Some(res.token.clone()));
        Ok(res)
    }

    /// Anonymous (guest) sign-in — creates a passwordless user and stores the
    /// session token. Link a real email + password later via
    /// [`update_me`](Self::update_me) (`email` + `password`).
    pub async fn sign_in_anonymously(&self) -> Result<LoginResponse> {
        let res: LoginResponse = self.http.post("/api/auth/anonymous", None).await?;
        self.http.set_token(Some(res.token.clone()));
        Ok(res)
    }

    /// Browser redirect URL that starts the OAuth2 flow for `provider`.
    pub fn oauth_url(&self, provider: &str) -> String {
        self.http.url(&format!("/api/auth/oauth/{provider}"))
    }

    // ---- MFA / TOTP ----

    /// Begin TOTP enrollment — returns the secret + otpauth URI to scan.
    pub async fn mfa_enable(&self) -> Result<MfaSetup> {
        self.http.post("/api/auth/mfa/enable", None).await
    }

    /// Confirm enrollment with a current TOTP code.
    pub async fn mfa_confirm(&self, code: &str) -> Result<Value> {
        self.http
            .post("/api/auth/mfa/confirm", json_body(json!({ "code": code })))
            .await
    }

    /// Disable MFA with a current TOTP code.
    pub async fn mfa_disable(&self, code: &str) -> Result<Value> {
        self.http
            .post("/api/auth/mfa/disable", json_body(json!({ "code": code })))
            .await
    }
}
