//! Tests that need a real service graph.

mod accounts;
mod attachments;
mod blueprint_bundle;
mod blueprints;
mod cards;
mod connections;
mod harness;
mod modules;
mod plugins;
mod realm_import;
mod roles;
mod tokenizers;
mod turns;

pub(crate) use harness::Harness;
