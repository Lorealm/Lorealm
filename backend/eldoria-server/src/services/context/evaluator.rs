use std::collections::HashMap;

use thiserror::Error;
use uuid::Uuid;

use crate::services::conversation::ChatMessage;

use super::types::{ContextNode, ContextTemplate, NodeContent, PromptRole};

/// Trait for tokenizers
pub trait Tokenizer: Send + Sync {
    fn count(&self, input: &str) -> usize;
}

/// Simple whitespace-based tokenizer for testing
#[derive(Debug, Default)]
pub struct SimpleTokenizer;

impl Tokenizer for SimpleTokenizer {
    fn count(&self, input: &str) -> usize {
        // Rough approximation: ~4 characters per token
        (input.len() + 3) / 4
    }
}

#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error("Unknown source: {0}")]
    UnknownSource(String),
    #[error("Source error: {0}")]
    Source(#[from] SourceError),
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Missing required data: {0}")]
    MissingData(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Result of evaluating a context template
#[derive(Debug)]
pub struct EvaluatedContext {
    /// The segments in order, ready to be combined
    pub segments: Vec<ContextSegment>,
    /// Total token count (estimated)
    pub total_tokens: usize,
}

impl EvaluatedContext {
    /// Combine all segments into a single string
    pub fn to_string(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get segments grouped by role
    pub fn by_role(&self) -> HashMap<Option<PromptRole>, Vec<&ContextSegment>> {
        let mut result: HashMap<Option<PromptRole>, Vec<&ContextSegment>> = HashMap::new();
        for segment in &self.segments {
            result
                .entry(segment.role.clone())
                .or_default()
                .push(segment);
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct ContextSegment {
    pub source: SegmentSource,
    pub content: String,
    pub role: Option<PromptRole>,
    pub token_count: usize,
}

#[derive(Debug, Clone)]
pub enum SegmentSource {
    TextNode { node_id: Uuid },
    SourceNode { source_id: String, node_id: String },
    GroupWrapper { node_id: Uuid, is_prefix: bool },
}

/// Trait that source providers implement to generate their content
pub trait SourceProvider: Send + Sync {
    fn source_id(&self) -> &str;

    /// Generate content for a specific node
    fn generate(
        &self,
        node_id: &str,
        settings: &HashMap<String, serde_json::Value>,
        context: &EvaluationContext,
    ) -> Result<String, SourceError>;
}

/// Context passed to source providers during evaluation
pub struct EvaluationContext<'a> {
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub character_id: Option<Uuid>,
    pub messages: &'a [ChatMessage],
    // pub world_info_entries: &'a [WorldInfoEntry],
    // pub character: Option<&'a Character>,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(chat_id: Uuid, user_id: Uuid, messages: &'a [ChatMessage]) -> Self {
        Self {
            chat_id,
            user_id,
            character_id: None,
            messages,
        }
    }

    pub fn with_character(mut self, character_id: Uuid) -> Self {
        self.character_id = Some(character_id);
        self
    }
}

pub struct ContextEvaluator {
    providers: HashMap<String, Box<dyn SourceProvider>>,
    tokenizer: Box<dyn Tokenizer>,
}

impl ContextEvaluator {
    /// Create a new evaluator with the given tokenizer
    pub fn new(tokenizer: Box<dyn Tokenizer>) -> Self {
        Self {
            providers: HashMap::new(),
            tokenizer,
        }
    }

    /// Create an evaluator with default simple tokenizer
    pub fn with_simple_tokenizer() -> Self {
        Self::new(Box::new(SimpleTokenizer))
    }

    /// Register a source provider
    pub fn register_provider(&mut self, provider: Box<dyn SourceProvider>) {
        self.providers
            .insert(provider.source_id().to_string(), provider);
    }

    /// Evaluate a template to produce the final context
    pub fn evaluate(
        &self,
        template: &ContextTemplate,
        eval_context: &EvaluationContext,
    ) -> Result<EvaluatedContext, EvaluationError> {
        let mut segments = Vec::new();

        self.evaluate_nodes(&template.nodes, eval_context, &mut segments)?;

        let total_tokens = segments.iter().map(|s| s.token_count).sum();

        Ok(EvaluatedContext {
            segments,
            total_tokens,
        })
    }

    fn evaluate_nodes(
        &self,
        nodes: &[ContextNode],
        eval_context: &EvaluationContext,
        segments: &mut Vec<ContextSegment>,
    ) -> Result<(), EvaluationError> {
        for node in nodes {
            if !node.enabled {
                continue;
            }

            match &node.content {
                NodeContent::Text { content, role } => {
                    if !content.trim().is_empty() {
                        let token_count = self.tokenizer.count(content);
                        segments.push(ContextSegment {
                            source: SegmentSource::TextNode { node_id: node.id },
                            content: content.clone(),
                            role: role.clone(),
                            token_count,
                        });
                    }
                }

                NodeContent::Source {
                    source_id,
                    node_id,
                    settings,
                } => {
                    let provider = self
                        .providers
                        .get(source_id)
                        .ok_or_else(|| EvaluationError::UnknownSource(source_id.clone()))?;

                    let content = provider.generate(node_id, settings, eval_context)?;

                    if !content.trim().is_empty() {
                        let token_count = self.tokenizer.count(&content);
                        segments.push(ContextSegment {
                            source: SegmentSource::SourceNode {
                                source_id: source_id.clone(),
                                node_id: node_id.clone(),
                            },
                            content,
                            role: None,
                            token_count,
                        });
                    }
                }

                NodeContent::Group {
                    children,
                    prefix,
                    suffix,
                    always_include_wrappers,
                } => {
                    let start_idx = segments.len();

                    self.evaluate_nodes(children, eval_context, segments)?;

                    let has_content = segments.len() > start_idx;

                    if has_content || *always_include_wrappers {
                        // Insert prefix at start of group content
                        if let Some(prefix_text) = prefix {
                            if !prefix_text.trim().is_empty() {
                                let token_count = self.tokenizer.count(prefix_text);
                                segments.insert(
                                    start_idx,
                                    ContextSegment {
                                        source: SegmentSource::GroupWrapper {
                                            node_id: node.id,
                                            is_prefix: true,
                                        },
                                        content: prefix_text.clone(),
                                        role: None,
                                        token_count,
                                    },
                                );
                            }
                        }

                        // Append suffix at end
                        if let Some(suffix_text) = suffix {
                            if !suffix_text.trim().is_empty() {
                                let token_count = self.tokenizer.count(suffix_text);
                                segments.push(ContextSegment {
                                    source: SegmentSource::GroupWrapper {
                                        node_id: node.id,
                                        is_prefix: false,
                                    },
                                    content: suffix_text.clone(),
                                    role: None,
                                    token_count,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockChatHistoryProvider;

    impl SourceProvider for MockChatHistoryProvider {
        fn source_id(&self) -> &str {
            "chat_history"
        }

        fn generate(
            &self,
            _node_id: &str,
            _settings: &HashMap<String, serde_json::Value>,
            context: &EvaluationContext,
        ) -> Result<String, SourceError> {
            // Simple mock that just returns a fixed string based on message count
            let count = context.messages.len();
            if count == 0 {
                Ok(String::new())
            } else {
                Ok(format!("[{} messages in history]", count))
            }
        }
    }

    fn create_test_evaluator() -> ContextEvaluator {
        let mut evaluator = ContextEvaluator::with_simple_tokenizer();
        evaluator.register_provider(Box::new(MockChatHistoryProvider));
        evaluator
    }

    #[test]
    fn test_evaluate_text_node() {
        let evaluator = ContextEvaluator::with_simple_tokenizer();

        let mut template = ContextTemplate::new("Test", Uuid::new_v4());
        template.nodes.push(ContextNode::new_text(
            "System",
            "You are helpful",
            Some(PromptRole::System),
        ));

        let context = EvaluationContext::new(Uuid::new_v4(), Uuid::new_v4(), &[]);

        let result = evaluator.evaluate(&template, &context).unwrap();

        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].content, "You are helpful");
        assert_eq!(result.segments[0].role, Some(PromptRole::System));
    }

    #[test]
    fn test_evaluate_disabled_node() {
        let evaluator = ContextEvaluator::with_simple_tokenizer();

        let mut template = ContextTemplate::new("Test", Uuid::new_v4());
        let mut node = ContextNode::new_text("System", "You are helpful", None);
        node.enabled = false;
        template.nodes.push(node);

        let context = EvaluationContext::new(Uuid::new_v4(), Uuid::new_v4(), &[]);

        let result = evaluator.evaluate(&template, &context).unwrap();

        assert!(result.segments.is_empty());
    }

    #[test]
    fn test_evaluate_group_with_wrappers() {
        let evaluator = ContextEvaluator::with_simple_tokenizer();

        let mut template = ContextTemplate::new("Test", Uuid::new_v4());

        let mut group = ContextNode::new_group("Character");
        if let NodeContent::Group {
            children,
            prefix,
            suffix,
            ..
        } = &mut group.content
        {
            *prefix = Some("<character>".into());
            *suffix = Some("</character>".into());
            children.push(ContextNode::new_text("Name", "Alice", None));
        }
        template.nodes.push(group);

        let context = EvaluationContext::new(Uuid::new_v4(), Uuid::new_v4(), &[]);

        let result = evaluator.evaluate(&template, &context).unwrap();

        assert_eq!(result.segments.len(), 3);
        assert_eq!(result.segments[0].content, "<character>");
        assert_eq!(result.segments[1].content, "Alice");
        assert_eq!(result.segments[2].content, "</character>");
    }

    #[test]
    fn test_evaluate_source_node() {
        let evaluator = create_test_evaluator();

        let mut template = ContextTemplate::new("Test", Uuid::new_v4());
        template
            .nodes
            .push(ContextNode::new_source("History", "chat_history", "main"));

        // Create mock messages using the actual ChatMessage constructors
        let messages = vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let context = EvaluationContext::new(Uuid::new_v4(), Uuid::new_v4(), &messages);

        let result = evaluator.evaluate(&template, &context).unwrap();

        assert_eq!(result.segments.len(), 1);
        assert!(result.segments[0].content.contains("2 messages"));
    }

    #[test]
    fn test_evaluate_unknown_source() {
        let evaluator = ContextEvaluator::with_simple_tokenizer();

        let mut template = ContextTemplate::new("Test", Uuid::new_v4());
        template
            .nodes
            .push(ContextNode::new_source("Unknown", "nonexistent", "main"));

        let context = EvaluationContext::new(Uuid::new_v4(), Uuid::new_v4(), &[]);

        let result = evaluator.evaluate(&template, &context);

        assert!(matches!(result, Err(EvaluationError::UnknownSource(_))));
    }
}