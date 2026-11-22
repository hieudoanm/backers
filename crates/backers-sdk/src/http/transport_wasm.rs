//! WASM transport — `fetch` via `web-sys` (Node ≥22 / browsers).

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Blob, BlobPropertyBag, FormData, Headers, Request, RequestInit, RequestMode, Response,
};

use crate::error::{BackersError, Result};
use crate::http::{Body, Inner, MultipartField};

fn js_err(context: &str) -> impl Fn(JsValue) -> BackersError + '_ {
    move |e| BackersError::Transport(format!("{context}: {e:?}"))
}

/// Sends the request and returns `(status, body_text)`.
pub(crate) async fn send(
    _inner: &Inner,
    method: &str,
    url: &str,
    headers: Vec<(String, String)>,
    body: Option<Body>,
) -> Result<(u16, String)> {
    let window = web_sys::window().ok_or_else(|| {
        BackersError::Transport("no global window/document (not a browser or Node ≥22)".into())
    })?;

    let init = RequestInit::new();
    init.set_method(method);
    init.set_mode(RequestMode::Cors);

    let header_map = Headers::new().map_err(js_err("headers"))?;
    for (name, value) in &headers {
        header_map.set(name, value).map_err(js_err("set header"))?;
    }
    init.set_headers(&header_map);

    match body {
        None => {}
        Some(Body::Json(value)) => {
            let text = serde_json::to_string(&value)
                .map_err(|e| BackersError::Codec(format!("encode body: {e}")))?;
            init.set_body_opt_str(Some(&text));
        }
        Some(Body::Multipart(form)) => {
            let data = FormData::new().map_err(js_err("form data"))?;
            for field in form.fields {
                match field {
                    MultipartField::Text { name, value } => {
                        data.append_with_str(&name, &value)
                            .map_err(js_err("append field"))?;
                    }
                    MultipartField::File {
                        name,
                        filename,
                        data: bytes,
                        content_type,
                    } => {
                        let arr = js_sys::Uint8Array::from(bytes.as_slice());
                        let parts = js_sys::Array::of1(&arr);
                        let blob = match content_type {
                            Some(ct) => {
                                let bag = BlobPropertyBag::new();
                                bag.set_type(&ct);
                                Blob::new_with_u8_array_sequence_and_options(&parts, &bag)
                                    .map_err(js_err("build blob"))?
                            }
                            None => Blob::new_with_u8_array_sequence(&parts)
                                .map_err(js_err("build blob"))?,
                        };
                        data.append_with_blob_and_filename(&name, &blob, &filename)
                            .map_err(js_err("append file"))?;
                    }
                }
            }
            init.set_body_opt_form_data(Some(&data));
        }
    }

    let request = Request::new_with_str_and_init(url, &init).map_err(js_err("build request"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(js_err("fetch"))?;
    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| BackersError::Transport("fetch did not return a Response".into()))?;
    let status = resp.status() as u16;

    let text_promise = resp.text().map_err(js_err("read body"))?;
    let text_value = JsFuture::from(text_promise)
        .await
        .map_err(js_err("read body"))?;
    let text = text_value.as_string().unwrap_or_default();
    Ok((status, text))
}
