//! Record + collection clients — `/api/collections/{name}/records*` and
//! `/api/collections*`.

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::Result;
use crate::http::{HttpClient, json_body, json_of};
use crate::types::{Collection, CollectionRules, ListOptions, RecordData, RecordsPage};

pub(crate) fn url_encode(id: &str) -> String {
    // encodeURIComponent equivalent for path segments.
    let mut out = String::with_capacity(id.len());
    for b in id.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn records_path(name: &str) -> String {
    format!("/api/collections/{}/records", url_encode(name))
}

fn collection_query(opts: &ListOptions) -> String {
    let mut pairs: Vec<(&str, String)> = Vec::new();
    if let Some(v) = opts.page {
        pairs.push(("page", v.to_string()));
    }
    if let Some(v) = opts.per_page {
        pairs.push(("per_page", v.to_string()));
    }
    if let Some(v) = &opts.search {
        pairs.push(("search", v.clone()));
    }
    if let Some(v) = &opts.filter {
        pairs.push(("filter", v.clone()));
    }
    if let Some(v) = &opts.sort {
        pairs.push(("sort", v.clone()));
    }
    if let Some(v) = &opts.expand {
        pairs.push(("expand", v.clone()));
    }
    if pairs.is_empty() {
        String::new()
    } else {
        let qs = pairs
            .iter()
            .map(|(k, v)| format!("{k}={}", url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("?{qs}")
    }
}

/// CRUD + listing for one collection.
#[derive(Debug, Clone)]
pub struct CollectionClient {
    http: HttpClient,
    /// Collection name.
    pub name: String,
}

impl CollectionClient {
    pub(crate) fn new(http: HttpClient, name: impl Into<String>) -> Self {
        CollectionClient {
            http,
            name: name.into(),
        }
    }

    /// One page of records matching `opts`.
    pub async fn list(&self, opts: &ListOptions) -> Result<RecordsPage> {
        self.http
            .get(&format!(
                "{}{}",
                records_path(&self.name),
                collection_query(opts)
            ))
            .await
    }

    /// Fetches every page matching `opts` (bounded by `max_pages`).
    pub async fn list_all(&self, opts: &ListOptions, max_pages: i64) -> Result<Vec<RecordData>> {
        let per_page = opts.per_page.unwrap_or(200);
        let mut out = Vec::new();
        for page in 1..=max_pages {
            let mut page_opts = opts.clone();
            page_opts.page = Some(page);
            page_opts.per_page = Some(per_page);
            let res = self.list(&page_opts).await?;
            let done = res.records.is_empty() || page >= res.total_pages;
            out.extend(res.records);
            if done {
                break;
            }
        }
        Ok(out)
    }

    /// Fetches one record by id.
    pub async fn get(&self, id: &str) -> Result<RecordData> {
        self.http
            .get(&format!("{}/{}", records_path(&self.name), url_encode(id)))
            .await
    }

    /// Creates a record; `data` is the schemaless document. Optional `id`
    /// sets the record id (generated server-side when absent).
    pub async fn create(&self, data: Value, id: Option<&str>) -> Result<RecordData> {
        let mut body = Map::new();
        body.insert("data".into(), data);
        if let Some(id) = id {
            body.insert("id".into(), json!(id));
        }
        self.http
            .post(&records_path(&self.name), json_body(Value::Object(body)))
            .await
    }

    /// Replaces the record's `data` document (send the complete desired object).
    pub async fn update(&self, id: &str, data: Value) -> Result<RecordData> {
        self.http
            .patch(
                &format!("{}/{}", records_path(&self.name), url_encode(id)),
                json_body(json!({ "data": data })),
            )
            .await
    }

    /// Deletes one record by id.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.http
            .delete(&format!("{}/{}", records_path(&self.name), url_encode(id)))
            .await
    }
}

/// Admin-level collection management (`/api/collections*`).
#[derive(Debug, Clone)]
pub struct CollectionsClient {
    http: HttpClient,
}

impl CollectionsClient {
    pub(crate) fn new(http: HttpClient) -> Self {
        CollectionsClient { http }
    }

    /// Lists every collection.
    pub async fn list(&self) -> Result<Vec<Collection>> {
        self.http.get("/api/collections").await
    }

    /// Fetches one collection by name.
    pub async fn get(&self, name: &str) -> Result<Collection> {
        self.http
            .get(&format!("/api/collections/{}", url_encode(name)))
            .await
    }

    /// Creates a collection; `schema` is a JSON-schema string (`"{}"` = schemaless).
    pub async fn create(&self, name: &str, schema: &str) -> Result<Collection> {
        #[derive(Serialize)]
        struct CreateBody<'a> {
            name: &'a str,
            schema: &'a str,
        }
        self.http
            .post("/api/collections", json_of(&CreateBody { name, schema })?)
            .await
    }

    /// Updates schema and/or the five rule expressions; `None` fields are omitted.
    pub async fn update(
        &self,
        name: &str,
        rules: &CollectionRules,
        schema: Option<&str>,
    ) -> Result<Collection> {
        #[derive(Serialize)]
        struct UpdateBody<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            schema: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "list_rule")]
            list_rule: Option<&'a String>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "view_rule")]
            view_rule: Option<&'a String>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "create_rule")]
            create_rule: Option<&'a String>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "update_rule")]
            update_rule: Option<&'a String>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "delete_rule")]
            delete_rule: Option<&'a String>,
        }
        let body = UpdateBody {
            schema,
            list_rule: rules.list_rule.as_ref(),
            view_rule: rules.view_rule.as_ref(),
            create_rule: rules.create_rule.as_ref(),
            update_rule: rules.update_rule.as_ref(),
            delete_rule: rules.delete_rule.as_ref(),
        };
        self.http
            .patch(
                &format!("/api/collections/{}", url_encode(name)),
                json_of(&body)?,
            )
            .await
    }

    /// Deletes a collection and every record it holds.
    pub async fn delete(&self, name: &str) -> Result<()> {
        self.http
            .delete(&format!("/api/collections/{}", url_encode(name)))
            .await
    }
}
