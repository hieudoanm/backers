//! Native transport — `reqwest` async client.

use crate::error::{BlazersError, Result};
use crate::http::{Body, Inner, MultipartField};

/// Sends the request and returns `(status, body_text)`.
pub(crate) async fn send(
    inner: &Inner,
    method: &str,
    url: &str,
    headers: Vec<(String, String)>,
    body: Option<Body>,
) -> Result<(u16, String)> {
    let mut builder = inner.client.request(
        method
            .parse()
            .map_err(|e| BlazersError::Codec(format!("bad method '{method}': {e}")))?,
        url,
    );
    for (name, value) in &headers {
        builder = builder.header(name.as_str(), value.as_str());
    }

    builder = match body {
        None => builder,
        Some(Body::Json(value)) => builder.json(&value),
        Some(Body::Multipart(form)) => {
            let mut mp = reqwest::multipart::Form::new();
            for field in form.fields {
                match field {
                    MultipartField::Text { name, value } => {
                        mp = mp.text(name, value);
                    }
                    MultipartField::File {
                        name,
                        filename,
                        data,
                        content_type,
                    } => {
                        let mut part = reqwest::multipart::Part::bytes(data).file_name(filename);
                        if let Some(ct) = content_type {
                            part = part.mime_str(&ct).map_err(|e| {
                                BlazersError::Codec(format!("bad content type '{ct}': {e}"))
                            })?;
                        }
                        mp = mp.part(name, part);
                    }
                }
            }
            builder.multipart(mp)
        }
    };

    let res = builder
        .send()
        .await
        .map_err(|e| BlazersError::Transport(e.to_string()))?;
    let status = res.status().as_u16();
    let text = res
        .text()
        .await
        .map_err(|e| BlazersError::Transport(e.to_string()))?;
    Ok((status, text))
}
