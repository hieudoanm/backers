use super::*;

impl<'a> Server<'a> {
    /// handle_message decodes one frame and dispatches it, writing the reply.
    /// An undecodable frame produces a parse error. A notification — a frame
    /// with no id — is never answered whatever the method, because answering
    /// one desynchronises the client. That check precedes the version check so
    /// a notification which is also malformed stays silent.
    pub(super) fn handle_message(&mut self, raw: &[u8]) {
        if let Some(response) = self.response_for(raw) {
            self.write(response);
        }
    }

    /// response_for returns the reply frame for one request, or `None` for
    /// notifications. Shared by the stdio loop (write) and the HTTP transport
    /// (serialize).
    pub fn response_for(&mut self, raw: &[u8]) -> Option<protocol::Response> {
        let request: Request = match serde_json::from_slice(raw) {
            Ok(request) => request,
            Err(err) => {
                return Some(protocol::error_response(
                    serde_json::Value::Null,
                    protocol::error_code::PARSE,
                    format!("parse error: {err}"),
                ));
            }
        };

        if request.is_notification() {
            let _ = writeln!(
                self.diagnostics,
                "ignoring notification for method {}",
                request.method.as_deref().unwrap_or("<missing>")
            );
            return None;
        }
        if request.jsonrpc.as_deref() != Some(protocol::JSONRPC_VERSION) {
            return Some(protocol::error_response(
                request.id.clone().unwrap_or(serde_json::Value::Null),
                protocol::error_code::INVALID_REQUEST,
                "invalid jsonrpc version",
            ));
        }

        let id = request.id.clone().unwrap_or(serde_json::Value::Null);
        match request.method.as_deref() {
            Some("initialize") => Some(protocol::success_response(
                id,
                protocol::initialize_result(request.params.as_ref(), SERVER_NAME, SERVER_VERSION),
            )),
            Some("ping") => Some(protocol::success_response(id, serde_json::json!({}))),
            Some("tools/list") => {
                let mut value =
                    serde_json::to_value(self.list_result()).unwrap_or(serde_json::Value::Null);
                if let Some(tools) = value.get_mut("tools").and_then(|t| t.as_array_mut()) {
                    for tool in tools {
                        if tool.get("title").is_none()
                            && let Some(name) = tool.get("name").and_then(|n| n.as_str())
                        {
                            tool["title"] = serde_json::json!(protocol::titleize(name));
                        }
                    }
                }
                Some(protocol::success_response(id, value))
            }
            Some("tools/call") => self.call_response(id, request.params.as_ref()),
            Some(other) => Some(protocol::error_response(
                id,
                protocol::error_code::METHOD_NOT_FOUND,
                format!("method not found: {other}"),
            )),
            None => Some(protocol::error_response(
                id,
                protocol::error_code::INVALID_REQUEST,
                "invalid request: method is required",
            )),
        }
    }

    /// list_result returns the registered tools sorted by name, so a client sees
    /// a stable order across runs.
    fn list_result(&self) -> protocol::ListToolsResult {
        let mut tools = self.tools.clone();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        protocol::ListToolsResult { tools }
    }

    /// call_response decodes the call params and runs the named handler,
    /// returning the reply frame. Absent or null params are treated as an
    /// empty object so a call with no params names no tool rather than
    /// failing to decode.
    fn call_response(
        &mut self,
        id: serde_json::Value,
        params: Option<&serde_json::Value>,
    ) -> Option<protocol::Response> {
        let call: protocol::ToolCallParams = match protocol::object_or_empty(params) {
            Ok(call) => call,
            Err(message) => {
                return Some(protocol::error_response(
                    id,
                    protocol::error_code::INVALID_PARAMS,
                    format!("invalid params: {message}"),
                ));
            }
        };

        let handler = self
            .handlers
            .iter()
            .find(|(name, _)| *name == call.name)
            .map(|(_, handler)| handler);

        match handler {
            None => Some(protocol::error_response(
                id,
                protocol::error_code::METHOD_NOT_FOUND,
                format!("tool not found: {}", call.name),
            )),
            Some(handler) => {
                let result = handler(&call.arguments_json());
                match serde_json::to_value(result) {
                    Ok(value) => Some(protocol::success_response(id, value)),
                    Err(err) => Some(protocol::error_response(
                        id,
                        protocol::error_code::INTERNAL,
                        format!("could not encode tool result: {err}"),
                    )),
                }
            }
        }
    }
}
