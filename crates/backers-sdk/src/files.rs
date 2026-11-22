//! Buckets + file upload/download (`/api/buckets/*`).

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::Result;
use crate::http::{Body, HttpClient, Multipart, json_body, json_of};
use crate::types::{Bucket, FileRecord, FileToken, FilesPage};

/// Options for `POST .../files/{id}/token`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileTokenOptions {
    /// When set together with `record`, the token is only issued if the
    /// caller passes the record's `view_rule` — mirrors PocketBase file
    /// tokens bound to records.
    pub collection: Option<String>,
    pub record: Option<String>,
}

/// Buckets + file client.
#[derive(Debug, Clone)]
pub struct FilesClient {
    http: HttpClient,
}

impl FilesClient {
    pub(crate) fn new(http: HttpClient) -> Self {
        FilesClient { http }
    }

    // ---- Buckets ----

    /// Lists every bucket.
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>> {
        self.http.get("/api/buckets").await
    }

    /// Creates a bucket; `is_public` allows unauthenticated downloads.
    pub async fn create_bucket(&self, name: &str, is_public: bool) -> Result<Bucket> {
        #[derive(Serialize)]
        struct BucketBody<'a> {
            name: &'a str,
            is_public: bool,
        }
        self.http
            .post("/api/buckets", json_of(&BucketBody { name, is_public })?)
            .await
    }

    /// Fetches one bucket by name.
    pub async fn get_bucket(&self, name: &str) -> Result<Bucket> {
        self.http
            .get(&format!(
                "/api/buckets/{}",
                crate::records::url_encode(name)
            ))
            .await
    }

    /// Deletes a bucket.
    pub async fn delete_bucket(&self, name: &str) -> Result<()> {
        self.http
            .delete(&format!(
                "/api/buckets/{}",
                crate::records::url_encode(name)
            ))
            .await
    }

    // ---- Files ----

    /// Lists files in `bucket` (1-based `page`, `per_page` page size).
    pub async fn list_files(&self, bucket: &str, page: i64, per_page: i64) -> Result<FilesPage> {
        self.http
            .get(&format!(
                "/api/buckets/{}/files?page={page}&per_page={per_page}",
                crate::records::url_encode(bucket)
            ))
            .await
    }

    /// Multipart upload of raw bytes. `content_type` sets the part MIME type
    /// (e.g. `image/png`); `None` lets the server detect it.
    pub async fn upload(
        &self,
        bucket: &str,
        filename: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<FileRecord> {
        let form = Multipart::new().file_field("file", filename, data, content_type);
        self.http
            .post(
                &format!("/api/buckets/{}/files", crate::records::url_encode(bucket)),
                Some(Body::Multipart(form)),
            )
            .await
    }

    /// URL that serves the file — works unauthenticated for public buckets;
    /// protected buckets need `token` (from [`file_token`](Self::file_token))
    /// or a Bearer header.
    pub fn download_url(&self, bucket: &str, file_id: &str, token: Option<&str>) -> String {
        let base = format!(
            "/api/buckets/{}/files/{}",
            crate::records::url_encode(bucket),
            crate::records::url_encode(file_id)
        );
        match token {
            Some(t) => format!(
                "{}?token={}",
                self.http.url(&base),
                crate::records::url_encode(t)
            ),
            None => self.http.url(&base),
        }
    }

    /// Fetches the file bytes (uses the session token when set).
    /// Returns raw bytes on every platform.
    pub async fn download(
        &self,
        bucket: &str,
        file_id: &str,
        token: Option<&str>,
    ) -> Result<Vec<u8>> {
        download_bytes(&self.http, &self.download_url(bucket, file_id, token)).await
    }

    /// Issues a ~1h download token for a protected file (auth required).
    pub async fn file_token(
        &self,
        bucket: &str,
        file_id: &str,
        opts: &FileTokenOptions,
    ) -> Result<FileToken> {
        let body = match (&opts.collection, &opts.record) {
            (Some(c), Some(r)) => json_body(json!({ "collection": c, "record": r })),
            _ => None,
        };
        self.http
            .post(
                &format!(
                    "/api/buckets/{}/files/{}/token",
                    crate::records::url_encode(bucket),
                    crate::records::url_encode(file_id)
                ),
                body,
            )
            .await
    }

    /// Deletes one file.
    pub async fn delete_file(&self, bucket: &str, file_id: &str) -> Result<()> {
        self.http
            .delete(&format!(
                "/api/buckets/{}/files/{}",
                crate::records::url_encode(bucket),
                crate::records::url_encode(file_id)
            ))
            .await
    }
}

/// Shared download helper — fetches `url` with the client's auth token and
/// returns the raw body bytes.
#[cfg(not(target_arch = "wasm32"))]
async fn download_bytes(http: &HttpClient, url: &str) -> Result<Vec<u8>> {
    let mut req = http.inner.client.get(url);
    if let Some(token) = http.auth_token() {
        req = req.bearer_auth(token);
    }
    let res = req
        .send()
        .await
        .map_err(|e| crate::error::BackersError::Transport(e.to_string()))?;
    let status = res.status().as_u16();
    if !(200..300).contains(&status) {
        let text = res.text().await.unwrap_or_default();
        return Err(crate::error::BackersError::from_response(status, &text));
    }
    res.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| crate::error::BackersError::Transport(e.to_string()))
}

#[cfg(target_arch = "wasm32")]
async fn download_bytes(http: &HttpClient, url: &str) -> Result<Vec<u8>> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let window = web_sys::window()
        .ok_or_else(|| crate::error::BackersError::Transport("no window".into()))?;
    let init = RequestInit::new();
    init.set_method("GET");
    init.set_mode(RequestMode::Cors);
    if let Some(token) = http.auth_token() {
        let headers = web_sys::Headers::new()
            .map_err(|e| crate::error::BackersError::Transport(format!("{e:?}")))?;
        headers
            .set("authorization", &format!("Bearer {token}"))
            .map_err(|e| crate::error::BackersError::Transport(format!("{e:?}")))?;
        init.set_headers(&headers);
    }
    let request = Request::new_with_str_and_init(url, &init)
        .map_err(|e| crate::error::BackersError::Transport(format!("{e:?}")))?;
    let value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| crate::error::BackersError::Transport(format!("{e:?}")))?;
    let resp: Response = value
        .dyn_into()
        .map_err(|_| crate::error::BackersError::Transport("not a Response".into()))?;
    let status = resp.status() as u16;
    let buf_promise = resp
        .array_buffer()
        .map_err(|e| crate::error::BackersError::Transport(format!("{e:?}")))?;
    let buf = JsFuture::from(buf_promise)
        .await
        .map_err(|e| crate::error::BackersError::Transport(format!("{e:?}")))?;
    let arr = js_sys::Uint8Array::new(&buf);
    if !(200..300).contains(&status) {
        let text = String::from_utf8_lossy(&arr.to_vec()).to_string();
        return Err(crate::error::BackersError::from_response(status, &text));
    }
    Ok(arr.to_vec())
}
