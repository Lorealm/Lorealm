mod service;
mod types;
mod widgets;

pub use service::ChatService;
pub use types::*;
pub use widgets::{
    AttachmentSpec, InferEvent, WidgetInstance, WidgetRenderer, WidgetSink,
    register_attachment_commands, sanitize_widget_markers,
};
