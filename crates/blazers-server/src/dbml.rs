//! DBML conversion API — `/api/dbml/*`.
//!
//! Port of the Go `dbml serve` endpoints onto the main Blazers router. Accepts
//! `application/json` (`{"dbml": "…", "formats": ["sql","mermaid","svg"]}`) or
//! `multipart/form-data` (`file` upload or `dbml`/`json` fields) and returns
//! the requested renderings. Conversion is a pure function — no auth required,
//! matching `/api/health`.

use axum::{Json, extract::Request, http::HeaderMap};
use blazers_core::models::AppError;
use blazers_dbml::Project;
use serde::Deserialize;

/// JSON request body for the convert endpoints.
#[derive(Debug, Deserialize)]
pub struct ConvertRequest {
    /// DBML source content (required unless a `file` part is uploaded).
    #[serde(default)]
    pub dbml: String,
    /// Formats to render; defaults to `["sql", "mermaid"]`.
    #[serde(default)]
    pub formats: Vec<String>,
}

/// Shared request → (project, formats) pipeline for all convert handlers.
async fn load_request(
    headers: &HeaderMap,
    req: Request,
) -> Result<(Project, Vec<String>), AppError> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let (source, formats) = if content_type.starts_with("multipart/form-data") {
        multipart_source(req, &content_type).await?
    } else {
        let bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024)
            .await
            .map_err(|err| AppError::BadRequest(format!("read body: {err}")))?;
        let parsed: ConvertRequest = serde_json::from_slice(&bytes)
            .map_err(|err| AppError::BadRequest(format!("invalid JSON body: {err}")))?;
        (parsed.dbml, parsed.formats)
    };

    if source.trim().is_empty() {
        return Err(AppError::BadRequest("dbml field is required".into()));
    }
    let project = blazers_dbml::parse(source.as_bytes())
        .map_err(|err| AppError::BadRequest(format!("parse error: {err}")))?;
    let formats = if formats.is_empty() {
        vec!["sql".to_string(), "mermaid".to_string()]
    } else {
        formats
    };
    Ok((project, formats))
}

/// Extracts DBML source and optional formats from a multipart body.
///
/// Accepts a `file` upload (raw DBML), a `dbml` text field, or a `json` field
/// carrying the full request object — same contract as the Go server.
async fn multipart_source(
    req: Request,
    content_type: &str,
) -> Result<(String, Vec<String>), AppError> {
    let boundary = multer::parse_boundary(content_type)
        .map_err(|_| AppError::BadRequest("could not parse multipart boundary".into()))?;
    let stream = http_body_util::BodyDataStream::new(req.into_body());
    let mut multipart = multer::Multipart::new(stream, boundary);

    let mut source = String::new();
    let mut formats = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::BadRequest(format!("multipart error: {err}")))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|err| AppError::BadRequest(format!("read file: {err}")))?;
                source = String::from_utf8_lossy(&bytes).into_owned();
            }
            "dbml" => {
                source = field
                    .text()
                    .await
                    .map_err(|err| AppError::BadRequest(format!("read dbml field: {err}")))?;
            }
            "json" => {
                let text = field
                    .text()
                    .await
                    .map_err(|err| AppError::BadRequest(format!("read json field: {err}")))?;
                let parsed: ConvertRequest = serde_json::from_str(&text)
                    .map_err(|err| AppError::BadRequest(format!("invalid json field: {err}")))?;
                source = parsed.dbml;
                formats = parsed.formats;
            }
            _ => {}
        }
    }
    if source.is_empty() {
        return Err(AppError::BadRequest(
            "no dbml content found in multipart request".into(),
        ));
    }
    Ok((source, formats))
}

fn render(project: &Project, formats: &[String]) -> serde_json::Value {
    let mut response = serde_json::json!({});
    for format in formats {
        match format.to_lowercase().as_str() {
            "sql" => response["sql"] = project.to_sql().into(),
            "mermaid" => response["mermaid"] = project.to_mermaid().into(),
            "svg" => response["svg"] = project.to_svg().into(),
            _ => {}
        }
    }
    response
}

/// `POST /api/dbml/convert` — render the requested formats (default sql+mermaid).
pub async fn convert(
    headers: HeaderMap,
    req: Request,
) -> Result<Json<serde_json::Value>, AppError> {
    let (project, formats) = load_request(&headers, req).await?;
    Ok(Json(render(&project, &formats)))
}

/// `POST /api/dbml/convert/sql` — SQL only.
pub async fn convert_sql(
    headers: HeaderMap,
    req: Request,
) -> Result<Json<serde_json::Value>, AppError> {
    let (project, _) = load_request(&headers, req).await?;
    Ok(Json(serde_json::json!({"sql": project.to_sql()})))
}

/// `POST /api/dbml/convert/mermaid` — Mermaid only.
pub async fn convert_mermaid(
    headers: HeaderMap,
    req: Request,
) -> Result<Json<serde_json::Value>, AppError> {
    let (project, _) = load_request(&headers, req).await?;
    Ok(Json(serde_json::json!({"mermaid": project.to_mermaid()})))
}

/// `POST /api/dbml/parse` — JSON schema summary.
pub async fn parse_dbml(
    headers: HeaderMap,
    req: Request,
) -> Result<Json<serde_json::Value>, AppError> {
    let (project, _) = load_request(&headers, req).await?;
    let summary = blazers_dbml::summary::summarize(&project)
        .map_err(|err| AppError::Internal(format!("summarize: {err}")))?;
    Ok(Json(serde_json::json!({"summary": summary})))
}
