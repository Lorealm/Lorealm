mod client;
mod error;
mod providers;
mod streaming;
mod tools;
mod types;

pub use client::{Client, RequestBuilder};
pub use error::{Error, Result};
pub use providers::*;
pub use streaming::{DeltaStreamExt, collect_blocks};
pub use tools::{Tool, ToolRegistry, TypedTool};
pub use types::{
    CommonParams, ContentBlock, Delta, ImageSource, Message, ModelInfo, ModelPricing, Response,
    Role, ToolCallDelta, ToolDef, Usage,
};
