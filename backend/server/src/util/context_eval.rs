use std::collections::HashMap;

use kitoken::Kitoken;
use lorealm_storage::models::{Character, ConversationMessage};

use crate::{
    errors::{ContextError, TokenizerError},
    services::context::ContextTemplate,
};

/// A source of a certain piece of context
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextSource {
    /// The chat history
    Conversation,
    /// Anything produced by nodes in a Context Template.
    /// This only includes text nodes, group prefixes or other flat nodes.
    ContextTemplate,
    /// Context produced by a lorebook
    Lorebook,
    /// Character description, personality and scenario
    Character,
    /// System prompts, Weaver [`effect`] injections and other things
    /// that aren't otherwise listed
    ///
    /// weaver-effect: crate::services::commands::CommandEffect
    Miscellaneous,
    /// Anything produced by a plugin
    Plugin,
}

pub struct ContextEvalResult {
    messages: Vec<ConversationMessage>,
}

pub struct CountResult(HashMap<ContextSource, u32>);

/// Evaluates some context by conditionally:
/// - Applying context limits
/// - Merging/squashing messages
///
/// Note: This is applied *after* equivalent nodes in a [`ContextTemplate`]
pub struct ContextEvaluator {
    template: ContextTemplate,
    tokenizer: Kitoken,
    messages: Vec<ConversationMessage>,
    config: ContextConstructorConfig,
}

impl ContextEvaluator {
    pub fn builder(template: ContextTemplate) -> ContextConstructorBuilder {
        ContextConstructorBuilder::from_template(template)
    }

    pub fn evaluate(&self) -> Result<ContextEvalResult, ContextError> {
        Ok(ContextEvalResult {
            messages: self.truncate_chat_history()?,
        })
    }

    pub fn grouped_count(&self) -> Result<CountResult, ContextError> {
        let mut counts = HashMap::new();
        counts.insert(
            ContextSource::Conversation,
            self.messages
                .count_tokens(&self.tokenizer, &self.template)?,
        );

        Ok(CountResult(counts))
    }

    fn truncate_chat_history(&self) -> Result<Vec<ConversationMessage>, TokenizerError> {
        let mut initial = self.messages.clone();

        let Some(token_limit) = self.config.token_limit else {
            return Ok(initial);
        };

        initial.sort_by_key(|m| m.order);

        let mut truncated = Vec::new();
        let mut token_sum = 0u32;

        if self.config.ch_truncation == ChatHistoryTruncationMode::Oldest {
            for msg in initial.iter().rev() {
                let tokens = self
                    .tokenizer
                    .encode(msg.content.clone(), false)
                    .map_err(|e| TokenizerError::Tokenize(e.to_string()))?;
                let len = tokens.len() as u32;

                if token_sum + len > token_limit {
                    break;
                }

                token_sum += len;
                truncated.push(msg.clone());
            }
        }

        truncated.reverse();
        Ok(truncated)
    }
}

#[derive(Default, PartialEq)]
pub enum ChatHistoryTruncationMode {
    #[default]
    Oldest,
}

#[derive(Default)]
pub struct ContextConstructorConfig {
    squash_system_messages: bool,
    token_limit: Option<u32>,
    /// How the chat history is truncated
    ch_truncation: ChatHistoryTruncationMode,
}

#[derive(Default)]
pub struct ContextConstructorBuilder {
    tokenizer: Option<Kitoken>,
    template: Option<ContextTemplate>,
    messages: Vec<ConversationMessage>,
    config: ContextConstructorConfig,
}

impl ContextConstructorBuilder {
    pub fn from_template(template: ContextTemplate) -> Self {
        Self {
            template: Some(template),
            ..Default::default()
        }
    }

    pub fn messages(mut self, messages: Vec<ConversationMessage>) -> Self {
        self.messages = messages;
        self
    }

    pub fn squash_system_messages(mut self, squash: bool) -> Self {
        self.config.squash_system_messages = squash;
        self
    }

    pub fn token_limit(mut self, limit: u32) -> Self {
        self.config.token_limit = Some(limit);
        self
    }

    pub fn with_tokenizer(mut self, tokenizer: Kitoken) -> Self {
        self.tokenizer = Some(tokenizer);
        self
    }

    pub fn build(self) -> Result<ContextEvaluator, ContextError> {
        let Some(template) = self.template else {
            return Err(ContextError::NoActiveTemplate);
        };
        let Some(tokenizer) = self.tokenizer else {
            return Err(TokenizerError::Tokenize("No tokenizer was set.".to_string()).into());
        };
        Ok(ContextEvaluator {
            template,
            tokenizer,
            messages: self.messages,
            config: self.config,
        })
    }
}

trait CountableSource {
    fn count_tokens(
        &self,
        tokenizer: &Kitoken,
        template: &ContextTemplate,
    ) -> Result<u32, TokenizerError>;
}

impl CountableSource for Vec<ConversationMessage> {
    fn count_tokens(
        &self,
        tokenizer: &Kitoken,
        template: &ContextTemplate,
    ) -> Result<u32, TokenizerError> {
        if !template.is_source_node_active("chat_history", "main") {
            return Ok(0);
        }

        let sum = self
            .iter()
            .map(|m| {
                Ok(tokenizer
                    .encode(&m.content, false)
                    .map_err(|e| TokenizerError::Tokenize(e.to_string()))?
                    .len() as u32)
            })
            .sum::<Result<u32, TokenizerError>>()?;

        Ok(sum)
    }
}

impl CountableSource for Character {
    fn count_tokens(
        &self,
        tokenizer: &Kitoken,
        template: &ContextTemplate,
    ) -> Result<u32, TokenizerError> {
        let mut sum = 0;

        if !template.is_source_node_active("character", "definition") {
            sum += tokenizer
                .encode(&self.definition.description, false)
                .map_err(|e| TokenizerError::Tokenize(e.to_string()))?
                .len() as u32;
        }

        if !template.is_source_node_active("character", "personality") {
            sum += tokenizer
                .encode(&self.definition.personality, false)
                .map_err(|e| TokenizerError::Tokenize(e.to_string()))?
                .len() as u32;
        }

        if !template.is_source_node_active("character", "scenario") {
            sum += tokenizer
                .encode(&self.definition.scenario, false)
                .map_err(|e| TokenizerError::Tokenize(e.to_string()))?
                .len() as u32;
        }

        Ok(sum)
    }
}
