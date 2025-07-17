use std::sync::Arc;

use futures::Stream;
use serde::Deserialize;

use super::types::Message;

pub trait LLMProvider: 'static {
    fn generate(&self, messages: Vec<Message>) -> impl std::future::Future<Output = Result<Box<dyn LLMProviderResponse>, reqwest::Error>> + Send;
    fn generate_stream(self: Arc<Self>, messages: Vec<Message>) -> impl std::future::Future<Output = Result<impl Stream<Item = Result<Box<dyn LLMProviderResponse>, Box<dyn std::error::Error + Send + Sync>>> + Send + Unpin, Box<dyn std::error::Error + Send + Sync>>> + Send;
    fn cancel(&self);
}

pub trait LLMProviderResponse: 'static + Send {
    fn text(&self) -> String;
    fn json(&self) -> serde_json::Value;
    fn is_empty(&self) -> bool;
}