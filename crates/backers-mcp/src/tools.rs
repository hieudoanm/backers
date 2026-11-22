//! The Backers tool surface advertised over MCP.
//!
//! Names match the Go port so a client can switch languages without editing its
//! prompts, and the list is sorted so tools/list output is stable.

use super::handlers::*;
use super::protocol::Tool;

/// A tool paired with the handler that runs it. The name is kept beside the
/// handler so a renamed tool cannot silently keep its old wiring.
pub struct Registered {
    pub tool: Tool,
    pub handler: Box<dyn Fn(&str) -> crate::protocol::ToolResult>,
}

/// All tools, sorted by name. A tool with no arguments uses [Schema::empty];
/// one with arguments states every property and marks the mandatory ones.
pub fn register(db: &DbHandle) -> Vec<Registered> {
    let mut tools = db_tools::collection_tools(db);
    tools.extend(flags::flag_tools(db));
    tools.extend(db_tools::record_tools(db));
    tools.extend(dbml::dbml_tools());
    tools
}

mod db_tools;
mod dbml;
mod flags;
mod kv;

pub use kv::register_kv;
