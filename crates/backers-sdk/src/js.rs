//! JavaScript facade — `#[wasm_bindgen]` exports so the crate can be used
//! from Node ≥22 / browsers via the `pkg/` bundle produced by
//! `wasm-bindgen`/`wasm-pack` (see `package.json`).
//!
//! Mirrors the former TypeScript SDK surface: `new Backers({url})`,
//! `bz.auth.login({...})`, `bz.collection(name).list({...})`, `bz.realtime`,
//! etc. Async methods return `Promise`s; arguments/returns are plain JS
//! objects marshalled with `serde-wasm-bindgen`.

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::Backers as RustBackers;
use crate::auth::{Credentials, UpdateMeOptions};
use crate::error::BackersError;
use crate::files::FileTokenOptions;
use crate::http::ClientOptions;
use crate::types::{
    Collection, CollectionRules, FlagEvaluation, FlagPatch, FlagUpsert, ListOptions, RealtimeEvent,
    RecordsPage, User,
};

type JsResult<T> = std::result::Result<T, JsValue>;

fn to_js<T: Serialize>(v: &T) -> JsResult<JsValue> {
    serde_wasm_bindgen::to_value(v).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn from_js<T: DeserializeOwned>(v: JsValue) -> JsResult<T> {
    serde_wasm_bindgen::from_value(v)
        .map_err(|e| JsValue::from_str(&format!("invalid argument: {e}")))
}

fn opt_js<T: DeserializeOwned>(v: JsValue) -> JsResult<Option<T>> {
    if v.is_undefined() || v.is_null() {
        Ok(None)
    } else {
        from_js(v).map(Some)
    }
}

fn reject(e: BackersError) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn names_arg(v: JsValue) -> JsResult<Vec<String>> {
    opt_js::<Vec<String>>(v).map(|o| o.unwrap_or_default())
}

fn context_arg(v: JsValue) -> JsResult<HashMap<String, Value>> {
    opt_js::<HashMap<String, Value>>(v).map(|o| o.unwrap_or_default())
}

#[wasm_bindgen]
pub struct Backers {
    inner: RustBackers,
}

#[wasm_bindgen]
impl Backers {
    /// `new Backers({ url, token? })` — or `new Backers("http://…")`.
    #[wasm_bindgen(constructor)]
    pub fn new(opts: JsValue) -> Self {
        let (url, token) = if opts.is_string() {
            (opts.as_string().unwrap_or_default(), None::<String>)
        } else {
            #[derive(serde::Deserialize)]
            struct Opts {
                url: String,
                #[serde(default)]
                token: Option<String>,
            }
            let o: Opts = from_js(opts.clone()).unwrap_or(Opts {
                url: String::new(),
                token: None,
            });
            (o.url, o.token)
        };
        Backers {
            inner: RustBackers::new(ClientOptions {
                url,
                token,
                headers: HashMap::new(),
            }),
        }
    }

    #[wasm_bindgen(getter, js_name = auth)]
    pub fn auth_js(&self) -> AuthJs {
        AuthJs {
            inner: self.inner.auth.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = collections)]
    pub fn collections_js(&self) -> CollectionsJs {
        CollectionsJs {
            inner: self.inner.collections.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = files)]
    pub fn files_js(&self) -> FilesJs {
        FilesJs {
            inner: self.inner.files.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = flags)]
    pub fn flags_js(&self) -> FlagsJs {
        FlagsJs {
            inner: self.inner.flags.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = functions)]
    pub fn functions_js(&self) -> FunctionsJs {
        FunctionsJs {
            inner: self.inner.functions.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = realtime)]
    pub fn realtime_js(&self) -> RealtimeJs {
        RealtimeJs {
            inner: self.inner.realtime.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = http)]
    pub fn http_js(&self) -> HttpJs {
        HttpJs {
            inner: self.inner.http.clone(),
        }
    }

    /// Client for one collection's records.
    #[wasm_bindgen(js_name = collection)]
    pub fn collection_js(&self, name: &str) -> CollectionJs {
        CollectionJs {
            inner: self.inner.collection(name),
        }
    }

    /// `GET /api/health`.
    pub async fn health(&self) -> JsResult<JsValue> {
        self.inner
            .health()
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }
}

#[wasm_bindgen]
pub struct HttpJs {
    inner: crate::http::HttpClient,
}

#[wasm_bindgen]
impl HttpJs {
    /// Sets the bearer token sent on every request (`undefined`/`null` clears).
    #[wasm_bindgen(js_name = setToken)]
    pub fn set_token_js(&self, token: Option<String>) {
        self.inner.set_token(token);
    }

    /// Current bearer token, or `null`.
    #[wasm_bindgen(getter, js_name = authToken)]
    pub fn auth_token_js(&self) -> Option<String> {
        self.inner.auth_token()
    }

    /// Base URL the client was configured with.
    #[wasm_bindgen(getter, js_name = baseUrl)]
    pub fn base_url_js(&self) -> String {
        self.inner.base_url()
    }
}

#[wasm_bindgen]
pub struct AuthJs {
    inner: crate::auth::AuthClient,
}

#[wasm_bindgen]
impl AuthJs {
    /// `register({ email, password })` → user.
    pub async fn register(&self, opts: JsValue) -> JsResult<JsValue> {
        let creds: Credentials = from_js(opts)?;
        self.inner
            .register(&creds)
            .await
            .map(|u: User| to_js(&u))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `login({ email, password })` → `LoginResult` (user, or MFA challenge).
    pub async fn login(&self, opts: JsValue) -> JsResult<JsValue> {
        let creds: Credentials = from_js(opts)?;
        self.inner
            .login(&creds)
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Completes an MFA challenge: `mfaLogin({ email, password, code })`.
    #[wasm_bindgen(js_name = mfaLogin)]
    pub async fn mfa_login_js(&self, opts: JsValue) -> JsResult<JsValue> {
        #[derive(serde::Deserialize)]
        struct Mfa {
            email: String,
            password: String,
            code: String,
        }
        let m: Mfa = from_js(opts)?;
        self.inner
            .mfa_login(&m.email, &m.password, &m.code)
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Refreshes the session token.
    pub async fn refresh(&self) -> JsResult<JsValue> {
        self.inner
            .refresh()
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Clears the local token (no server call).
    pub fn logout(&self) {
        self.inner.logout();
    }

    /// Current user.
    pub async fn me(&self) -> JsResult<JsValue> {
        self.inner
            .me()
            .await
            .map(|u: User| to_js(&u))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `updateMe({ data?, password?, currentPassword?, email? })`.
    #[wasm_bindgen(js_name = updateMe)]
    pub async fn update_me_js(&self, opts: JsValue) -> JsResult<JsValue> {
        let o: UpdateMeOptions = from_js(opts)?;
        self.inner
            .update_me(&o)
            .await
            .map(|u: User| to_js(&u))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Sends a password-reset email.
    #[wasm_bindgen(js_name = forgotPassword)]
    pub async fn forgot_password_js(&self, email: &str) -> JsResult<JsValue> {
        self.inner
            .forgot_password(email)
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Completes a password reset.
    #[wasm_bindgen(js_name = resetPassword)]
    pub async fn reset_password_js(&self, token: &str, password: &str) -> JsResult<JsValue> {
        self.inner
            .reset_password(token, password)
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Sends a verification email.
    #[wasm_bindgen(js_name = requestVerification)]
    pub async fn request_verification_js(&self, email: &str) -> JsResult<JsValue> {
        self.inner
            .request_verification(email)
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Confirms an email-verification token.
    #[wasm_bindgen(js_name = confirmVerification)]
    pub async fn confirm_verification_js(&self, token: &str) -> JsResult<JsValue> {
        self.inner
            .confirm_verification(token)
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Sends a login OTP.
    #[wasm_bindgen(js_name = requestOtp)]
    pub async fn request_otp_js(&self, email: &str) -> JsResult<JsValue> {
        self.inner
            .request_otp(email)
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `authWithOtp(email, otp)` → login response (stores the token).
    #[wasm_bindgen(js_name = authWithOtp)]
    pub async fn auth_with_otp_js(&self, email: &str, otp: &str) -> JsResult<JsValue> {
        self.inner
            .auth_with_otp(email, otp)
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Anonymous sign-in (stores the token).
    #[wasm_bindgen(js_name = signInAnonymously)]
    pub async fn sign_in_anonymously_js(&self) -> JsResult<JsValue> {
        self.inner
            .sign_in_anonymously()
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// OAuth2 authorize URL for `provider` (e.g. `google`).
    #[wasm_bindgen(js_name = oauthUrl)]
    pub fn oauth_url_js(&self, provider: &str) -> String {
        self.inner.oauth_url(provider)
    }
}

#[wasm_bindgen]
pub struct CollectionJs {
    inner: crate::records::CollectionClient,
}

#[wasm_bindgen]
impl CollectionJs {
    /// `list({ page?, perPage?, search?, filter?, sort?, expand? })` → page.
    pub async fn list(&self, opts: JsValue) -> JsResult<JsValue> {
        let o: ListOptions = opt_js(opts)?.unwrap_or_default();
        self.inner
            .list(&o)
            .await
            .map(|p: RecordsPage| to_js(&p))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Every matching page (bounded), flattened.
    #[wasm_bindgen(js_name = listAll)]
    pub async fn list_all(&self, opts: JsValue, max_pages: i64) -> JsResult<JsValue> {
        let o: ListOptions = opt_js(opts)?.unwrap_or_default();
        self.inner
            .list_all(&o, max_pages)
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// One record by id.
    pub async fn get(&self, id: &str) -> JsResult<JsValue> {
        self.inner
            .get(id)
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `create(data, id?)`.
    pub async fn create(&self, data: JsValue, id: Option<String>) -> JsResult<JsValue> {
        let data: Value = from_js(data)?;
        self.inner
            .create(data, id.as_deref())
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `update(id, data)`.
    pub async fn update(&self, id: &str, data: JsValue) -> JsResult<JsValue> {
        let data: Value = from_js(data)?;
        self.inner
            .update(id, data)
            .await
            .map(|r| to_js(&r))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Deletes a record.
    pub async fn delete(&self, id: &str) -> JsResult<()> {
        self.inner.delete(id).await.map_err(reject)
    }
}

#[wasm_bindgen]
pub struct CollectionsJs {
    inner: crate::records::CollectionsClient,
}

fn schema_arg(v: JsValue) -> JsResult<Option<String>> {
    match opt_js::<Value>(v)? {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(other) => serde_json::to_string(&other)
            .map(Some)
            .map_err(|e| JsValue::from_str(&e.to_string())),
    }
}

#[wasm_bindgen]
impl CollectionsJs {
    /// All collections.
    pub async fn list(&self) -> JsResult<JsValue> {
        self.inner
            .list()
            .await
            .map(|v: Vec<Collection>| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// One collection by name.
    pub async fn get(&self, name: &str) -> JsResult<JsValue> {
        self.inner
            .get(name)
            .await
            .map(|c: Collection| to_js(&c))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `create(name, schema?)` — schema as JSON string or object.
    pub async fn create(&self, name: &str, schema: JsValue) -> JsResult<JsValue> {
        let schema = schema_arg(schema)?.unwrap_or_else(|| "{}".into());
        self.inner
            .create(name, &schema)
            .await
            .map(|c: Collection| to_js(&c))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `update(name, { schema?, listRule?, viewRule?, createRule?, updateRule?, deleteRule? })`.
    pub async fn update(&self, name: &str, opts: JsValue) -> JsResult<JsValue> {
        #[derive(serde::Deserialize, Default)]
        struct Opts {
            #[serde(default)]
            schema: Option<Value>,
            #[serde(flatten)]
            rules: CollectionRules,
        }
        let o: Opts = opt_js(opts)?.unwrap_or_default();
        let schema = match o.schema {
            None => None,
            Some(Value::String(s)) => Some(s),
            Some(other) => {
                Some(serde_json::to_string(&other).map_err(|e| JsValue::from_str(&e.to_string()))?)
            }
        };
        self.inner
            .update(name, &o.rules, schema.as_deref())
            .await
            .map(|c: Collection| to_js(&c))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Deletes a collection.
    pub async fn delete(&self, name: &str) -> JsResult<()> {
        self.inner.delete(name).await.map_err(reject)
    }
}

#[wasm_bindgen]
pub struct FilesJs {
    inner: crate::files::FilesClient,
}

#[wasm_bindgen]
impl FilesJs {
    /// All buckets.
    #[wasm_bindgen(js_name = listBuckets)]
    pub async fn list_buckets_js(&self) -> JsResult<JsValue> {
        self.inner
            .list_buckets()
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Creates a bucket.
    #[wasm_bindgen(js_name = createBucket)]
    pub async fn create_bucket_js(&self, name: &str, is_public: bool) -> JsResult<JsValue> {
        self.inner
            .create_bucket(name, is_public)
            .await
            .map(|b| to_js(&b))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// One bucket by name.
    #[wasm_bindgen(js_name = getBucket)]
    pub async fn get_bucket_js(&self, name: &str) -> JsResult<JsValue> {
        self.inner
            .get_bucket(name)
            .await
            .map(|b| to_js(&b))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Deletes a bucket.
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket_js(&self, name: &str) -> JsResult<()> {
        self.inner.delete_bucket(name).await.map_err(reject)
    }

    /// `listFiles(bucket, page?, perPage?)` → page of file records.
    #[wasm_bindgen(js_name = listFiles)]
    pub async fn list_files_js(
        &self,
        bucket: &str,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> JsResult<JsValue> {
        self.inner
            .list_files(bucket, page.unwrap_or(1), per_page.unwrap_or(50))
            .await
            .map(|p| to_js(&p))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `upload(bucket, file)` — accepts a `Blob`/`File` (browser) or any
    /// object with `name`, `type` and `arrayBuffer()`.
    pub async fn upload(&self, bucket: &str, file: JsValue) -> JsResult<JsValue> {
        use wasm_bindgen_futures::JsFuture;

        let obj: js_sys::Object = file.unchecked_into();
        let name = js_sys::Reflect::get(&obj, &JsValue::from_str("name"))
            .ok()
            .and_then(|v| v.as_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "file".into());
        let ty = js_sys::Reflect::get(&obj, &JsValue::from_str("type"))
            .ok()
            .and_then(|v| v.as_string())
            .filter(|s| !s.is_empty());
        let ab_fn = js_sys::Reflect::get(&obj, &JsValue::from_str("arrayBuffer"))
            .map_err(|_| JsValue::from_str("upload: argument is not a Blob/File"))?;
        let ab_fn: js_sys::Function = ab_fn
            .dyn_into()
            .map_err(|_| JsValue::from_str("upload: argument is not a Blob/File"))?;
        let promise = ab_fn
            .call0(&obj)
            .map_err(|e| JsValue::from_str(&format!("upload: arrayBuffer() failed: {e:?}")))?;
        let ab = JsFuture::from(js_sys::Promise::from(promise))
            .await
            .map_err(|e| JsValue::from_str(&format!("upload: arrayBuffer() rejected: {e:?}")))?;
        let ab: js_sys::ArrayBuffer = ab.dyn_into().map_err(|_| {
            JsValue::from_str("upload: arrayBuffer() did not return an ArrayBuffer")
        })?;
        let bytes = js_sys::Uint8Array::new(&ab).to_vec();
        self.inner
            .upload(bucket, &name, bytes, ty.as_deref())
            .await
            .map(|f| to_js(&f))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `uploadBytes(bucket, filename, bytes, contentType?)` — raw bytes path.
    #[wasm_bindgen(js_name = uploadBytes)]
    pub async fn upload_bytes(
        &self,
        bucket: &str,
        filename: &str,
        data: js_sys::Uint8Array,
        content_type: Option<String>,
    ) -> JsResult<JsValue> {
        self.inner
            .upload(bucket, filename, data.to_vec(), content_type.as_deref())
            .await
            .map(|f| to_js(&f))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Direct download URL (works for public buckets; protected needs `token`).
    #[wasm_bindgen(js_name = downloadUrl)]
    pub fn download_url_js(&self, bucket: &str, file_id: &str, token: Option<String>) -> String {
        self.inner.download_url(bucket, file_id, token.as_deref())
    }

    /// Fetches file bytes → `Uint8Array`.
    pub async fn download(
        &self,
        bucket: &str,
        file_id: &str,
        token: Option<String>,
    ) -> JsResult<js_sys::Uint8Array> {
        let bytes = self
            .inner
            .download(bucket, file_id, token.as_deref())
            .await
            .map_err(reject)?;
        Ok(js_sys::Uint8Array::from(bytes.as_slice()))
    }

    /// Issues a ~1h download token for a protected file.
    #[wasm_bindgen(js_name = fileToken)]
    pub async fn file_token_js(
        &self,
        bucket: &str,
        file_id: &str,
        opts: JsValue,
    ) -> JsResult<JsValue> {
        let o: FileTokenOptions = opt_js(opts)?.unwrap_or_default();
        self.inner
            .file_token(bucket, file_id, &o)
            .await
            .map(|t| to_js(&t))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Deletes a file.
    #[wasm_bindgen(js_name = deleteFile)]
    pub async fn delete_file_js(&self, bucket: &str, file_id: &str) -> JsResult<()> {
        self.inner
            .delete_file(bucket, file_id)
            .await
            .map_err(reject)
    }
}

#[wasm_bindgen]
pub struct FlagsJs {
    inner: crate::flags::FlagsClient,
}

#[wasm_bindgen]
impl FlagsJs {
    /// `list(names?)` — empty/undefined lists all flags.
    pub async fn list(&self, names: JsValue) -> JsResult<JsValue> {
        let names = names_arg(names)?;
        self.inner
            .list(&names)
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// One flag by name.
    pub async fn get(&self, name: &str) -> JsResult<JsValue> {
        self.inner
            .get(name)
            .await
            .map(|f| to_js(&f))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Creates/replaces a flag: `upsert({ name, enabled?, rules?, payload?, description? })`.
    pub async fn upsert(&self, opts: JsValue) -> JsResult<JsValue> {
        let f: FlagUpsert = from_js(opts)?;
        self.inner
            .upsert(&f)
            .await
            .map(|f| to_js(&f))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// `update(name, { enabled?, rules?, payload?, description? })`.
    pub async fn update(&self, name: &str, opts: JsValue) -> JsResult<JsValue> {
        let p: FlagPatch = from_js(opts)?;
        self.inner
            .update(name, &p)
            .await
            .map(|f| to_js(&f))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Deletes a flag.
    pub async fn remove(&self, name: &str) -> JsResult<()> {
        self.inner.remove(name).await.map_err(reject)
    }

    /// `evaluate(names?, context?)` → `{ [name]: { enabled, payload? } }`.
    pub async fn evaluate(&self, names: JsValue, context: JsValue) -> JsResult<JsValue> {
        let names = names_arg(names)?;
        let context = context_arg(context)?;
        self.inner
            .evaluate(&names, &context)
            .await
            .map(|m: HashMap<String, FlagEvaluation>| to_js(&m))
            .unwrap_or_else(|e| Err(reject(e)))
    }
}

#[wasm_bindgen]
pub struct FunctionsJs {
    inner: crate::functions::FunctionsClient,
}

#[wasm_bindgen]
impl FunctionsJs {
    /// All edge functions.
    pub async fn list(&self) -> JsResult<JsValue> {
        self.inner
            .list()
            .await
            .map(|v| to_js(&v))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Creates/updates a function's JS source.
    pub async fn upsert(&self, name: &str, code: &str) -> JsResult<JsValue> {
        self.inner
            .upsert(name, code)
            .await
            .map(|f| to_js(&f))
            .unwrap_or_else(|e| Err(reject(e)))
    }

    /// Deletes a function.
    pub async fn remove(&self, name: &str) -> JsResult<()> {
        self.inner.remove(name).await.map_err(reject)
    }

    /// Invokes `name(input)` → the handler's return value.
    pub async fn invoke(&self, name: &str, input: JsValue) -> JsResult<JsValue> {
        let input: Option<Value> = opt_js(input)?;
        let res: Value = self.inner.invoke(name, input).await.map_err(reject)?;
        to_js(&res)
    }
}

#[wasm_bindgen]
pub struct RealtimeJs {
    inner: crate::realtime::RealtimeClient,
}

#[wasm_bindgen]
impl RealtimeJs {
    /// Opens (or reuses) the socket; resolves immediately — frames queue
    /// until `open`.
    pub async fn connect(&self) -> JsResult<()> {
        self.inner.connect().await.map_err(reject)
    }

    /// Whether the socket is open.
    #[wasm_bindgen(getter)]
    pub fn connected(&self) -> bool {
        self.inner.connected()
    }

    /// `subscribe(collection, { filter? })`.
    #[wasm_bindgen(js_name = subscribe)]
    pub async fn subscribe_js(&self, collection: &str, opts: JsValue) -> JsResult<()> {
        #[derive(serde::Deserialize, Default)]
        struct Opts {
            filter: Option<String>,
        }
        let o: Opts = opt_js(opts)?.unwrap_or_default();
        self.inner
            .subscribe(collection, o.filter.as_deref(), |_| {})
            .await
            .map_err(reject)
    }

    /// Removes a subscription.
    pub async fn unsubscribe(&self, collection: &str) -> JsResult<()> {
        self.inner.unsubscribe(collection).await.map_err(reject)
    }

    /// `presenceJoin(channel, { state? })`.
    #[wasm_bindgen(js_name = presenceJoin)]
    pub async fn presence_join_js(&self, channel: &str, opts: JsValue) -> JsResult<()> {
        #[derive(serde::Deserialize, Default)]
        struct Opts {
            state: Option<Value>,
        }
        let o: Opts = opt_js(opts)?.unwrap_or_default();
        self.inner
            .presence_join(channel, o.state, |_| {})
            .await
            .map_err(reject)
    }

    /// Updates presence state on a joined channel.
    #[wasm_bindgen(js_name = presenceUpdate)]
    pub async fn presence_update_js(&self, channel: &str, state: JsValue) -> JsResult<()> {
        let state: Value = from_js(state)?;
        self.inner
            .presence_update(channel, state)
            .await
            .map_err(reject)
    }

    /// Leaves a presence channel.
    #[wasm_bindgen(js_name = presenceLeave)]
    pub async fn presence_leave_js(&self, channel: &str) -> JsResult<()> {
        self.inner.presence_leave(channel).await.map_err(reject)
    }

    /// Registers a listener for every realtime frame; returns an
    /// unsubscribe function.
    #[wasm_bindgen(js_name = onMessage)]
    pub fn on_message_js(&self, cb: &js_sys::Function) -> js_sys::Function {
        let cb = cb.clone();
        let id = self.inner.on_message(move |ev: RealtimeEvent| {
            let frame = to_js(&ev.to_value()).unwrap_or(JsValue::NULL);
            let _ = cb.call1(&JsValue::NULL, &frame);
        });
        let rt = self.inner.clone();
        let unsub = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            rt.off_message(id);
        }) as Box<dyn FnMut()>);
        let f: js_sys::Function = unsub.as_ref().unchecked_ref::<js_sys::Function>().clone();
        // Keep the closure alive for as long as JS may call the function.
        unsub.forget();
        f
    }

    /// Permanently closes the socket.
    pub fn close(&self) {
        self.inner.close();
    }
}
