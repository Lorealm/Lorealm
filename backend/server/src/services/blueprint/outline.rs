//! Reading a document back into the editor's block model.
//!
//! Expansion is lossy on purpose: it *runs* a `foreach`, so what comes out is
//! a flat cast with no memory of the loop that produced it. The editor needs
//! the other thing — the structure as written — and that only exists in the
//! parse tree, which the server builds on every save and then discards.
//!
//! This walks that tree and projects the subset the editor can represent.
//! Anything else is refused by name rather than silently flattened: a
//! hand-written document is not wrong, it is just not something the block
//! model can hold.

use lorealm_storage::models::{RealmResourceKind, Snowflake};
use serde::Serialize;
use weaver_lang::PathSegment;
use weaver_lang::ast::expr::{ArrayItem, ExprKind, ObjectItem, VariableRef};
use weaver_lang::ast::template::{NodeKind, Template};
use weaver_lang::ast::value::Value;
use weaver_lang::{Span, ValueDoc};

use crate::services::blueprint::InputSpec;

/// Why a document cannot be opened in the editor.
#[derive(Debug, Serialize)]
pub struct OutlineRefusal {
    /// What was expected, in the author's terms.
    pub message: String,
    /// Where in the source, when the offending node carried a span.
    pub from: Option<usize>,
    pub to: Option<usize>,
}

impl OutlineRefusal {
    fn at(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            from: Some(span.start),
            to: Some(span.end),
        }
    }

    fn bare(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            from: None,
            to: None,
        }
    }
}

type Refused<T> = Result<T, OutlineRefusal>;

/// The document as the editor holds it.
#[derive(Debug, Serialize)]
pub struct Outline {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub director: Option<DirectorBlock>,
    pub inputs: Vec<InputSpec>,
    pub cast: Vec<CastEntry>,
}

/// How the realm picks who acts next. `None` in a document means the default
/// rotation, which is what the editor shows as "round robin over everyone".
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DirectorBlock {
    RoundRobin { over: FilterBlock },
    Random { over: FilterBlock },
    Script { source: String },
}

#[derive(Debug, Default, Serialize)]
#[serde(tag = "match", rename_all = "snake_case")]
pub enum FilterBlock {
    #[default]
    All,
    Kind {
        kind: String,
    },
    Handles {
        handles: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CastEntry {
    Agent {
        agent: AgentBlock,
    },
    Foreach {
        var_name: String,
        over: Option<String>,
        body: Vec<AgentBlock>,
    },
}

#[derive(Debug, Default, Serialize)]
pub struct AgentBlock {
    pub handle: String,
    pub name: String,
    pub kind: String,
    pub character: Option<Binding>,
    pub template: Option<Binding>,
    pub persona: Option<Binding>,
    pub profile: Option<Binding>,
    pub lorebooks: Vec<Binding>,
    pub script: String,
    pub run_condition: String,
}

/// What fills one id-shaped field.
///
/// A resource carries no name: the document stores an id and nothing else, so
/// the client resolves the label from the catalogue it already has.
#[derive(Debug, Serialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum Binding {
    Resource {
        kind: RealmResourceKind,
        id: Snowflake,
    },
    Loop {
        #[serde(rename = "varName")]
        var_name: String,
    },
    Input {
        name: String,
    },
}

/// Either the block model, or why the document cannot be shown as one.
///
/// A refusal is a normal answer, not an error: the document is fine, it simply
/// says something the editor has no way to draw. The client shows the reason.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OutlineResult {
    Ok { outline: Outline },
    Refused { reason: OutlineRefusal },
}

impl From<Refused<Outline>> for OutlineResult {
    fn from(result: Refused<Outline>) -> Self {
        match result {
            Ok(outline) => Self::Ok { outline },
            Err(reason) => Self::Refused { reason },
        }
    }
}

/// Project a parsed document, or say why it cannot be.
pub fn outline(doc: &ValueDoc) -> Refused<Outline> {
    let inputs = doc.inputs.iter().map(InputSpec::of).collect();

    let ExprKind::ObjectLiteral(items) = &doc.value.node else {
        return Err(OutlineRefusal::at(
            doc.value.span,
            "the document's value must be an object",
        ));
    };

    let mut name = String::new();
    let mut description = String::new();
    let mut tags = Vec::new();
    let mut director = None;
    let mut cast = Vec::new();

    for item in items {
        let ObjectItem::Entry(entry) = item else {
            return Err(OutlineRefusal::bare(
                "a loop or conditional at the top level of the document",
            ));
        };
        let Some(key) = entry.static_key() else {
            return Err(OutlineRefusal::at(entry.key.span, "a computed key"));
        };
        match key {
            "name" => name = text(&entry.value.node, entry.value.span)?,
            "description" => description = text(&entry.value.node, entry.value.span)?,
            "agents" => cast = agents(&entry.value.node, entry.value.span)?,
            "tags" => tags = strings(&entry.value.node, entry.value.span)?,
            "director" => director = director_of(&entry.value.node, entry.value.span)?,
            // `scenarios` round-trips untouched because nothing authors it yet.
            _ => {}
        }
    }

    Ok(Outline {
        name,
        description,
        tags,
        director,
        inputs,
        cast,
    })
}

fn strings(expr: &ExprKind, span: Span) -> Refused<Vec<String>> {
    let ExprKind::ArrayLiteral(items) = expr else {
        return Err(OutlineRefusal::at(span, "`tags` must be an array"));
    };
    items
        .iter()
        .map(|item| match item {
            ArrayItem::Element(expr) => text(&expr.node, expr.span),
            _ => Err(OutlineRefusal::bare("a loop or conditional in `tags`")),
        })
        .collect()
}

fn director_of(expr: &ExprKind, span: Span) -> Refused<Option<DirectorBlock>> {
    if matches!(expr, ExprKind::Literal(Value::None)) {
        return Ok(None);
    }
    let ExprKind::ObjectLiteral(items) = expr else {
        return Err(OutlineRefusal::at(span, "`director` must be an object or null"));
    };

    let mut kind = String::new();
    let mut source = String::new();
    let mut over = FilterBlock::All;

    for item in items {
        let ObjectItem::Entry(entry) = item else {
            return Err(OutlineRefusal::bare("a loop or conditional in `director`"));
        };
        match entry.static_key() {
            Some("kind") => kind = text(&entry.value.node, entry.value.span)?,
            Some("source") => source = text(&entry.value.node, entry.value.span)?,
            Some("over") => over = filter_of(&entry.value.node, entry.value.span)?,
            _ => {}
        }
    }

    Ok(Some(match kind.as_str() {
        "random" => DirectorBlock::Random { over },
        "script" => DirectorBlock::Script { source },
        "round_robin" => DirectorBlock::RoundRobin { over },
        other => {
            return Err(OutlineRefusal::at(
                span,
                format!("`{other}` is not a director the editor knows"),
            ));
        }
    }))
}

fn filter_of(expr: &ExprKind, span: Span) -> Refused<FilterBlock> {
    let ExprKind::ObjectLiteral(items) = expr else {
        return Err(OutlineRefusal::at(span, "`over` must be an object"));
    };

    let mut matcher = String::new();
    let mut kind = String::new();
    let mut handles = Vec::new();

    for item in items {
        let ObjectItem::Entry(entry) = item else {
            return Err(OutlineRefusal::bare("a loop or conditional in `over`"));
        };
        match entry.static_key() {
            Some("match") => matcher = text(&entry.value.node, entry.value.span)?,
            Some("kind") => kind = text(&entry.value.node, entry.value.span)?,
            Some("handles") => handles = strings(&entry.value.node, entry.value.span)?,
            _ => {}
        }
    }

    Ok(match matcher.as_str() {
        "kind" => FilterBlock::Kind { kind },
        "handles" => FilterBlock::Handles { handles },
        _ => FilterBlock::All,
    })
}

fn agents(expr: &ExprKind, span: Span) -> Refused<Vec<CastEntry>> {
    let ExprKind::ArrayLiteral(items) = expr else {
        return Err(OutlineRefusal::at(span, "`agents` must be an array"));
    };

    items
        .iter()
        .map(|item| match item {
            ArrayItem::Element(expr) => Ok(CastEntry::Agent {
                agent: agent(&expr.node, expr.span)?,
            }),
            ArrayItem::ForEach(loop_) => {
                let body = loop_
                    .body
                    .iter()
                    .map(|item| match item {
                        ArrayItem::Element(expr) => agent(&expr.node, expr.span),
                        _ => Err(OutlineRefusal::bare(
                            "a nested loop or conditional inside a `foreach`",
                        )),
                    })
                    .collect::<Refused<Vec<_>>>()?;

                Ok(CastEntry::Foreach {
                    var_name: loop_.binding.clone(),
                    over: walked(&loop_.iterable.node),
                    body,
                })
            }
            ArrayItem::If(_) => Err(OutlineRefusal::at(
                span,
                "a conditional in the cast, which the editor cannot show",
            )),
        })
        .collect()
}

/// The input a `foreach` walks, when it walks one plainly.
fn walked(expr: &ExprKind) -> Option<String> {
    match expr {
        ExprKind::Variable(VariableRef {
            scope: Some(scope),
            name,
            path,
        }) if scope == "input" && path.is_empty() => Some(name.clone()),
        _ => None,
    }
}

fn agent(expr: &ExprKind, span: Span) -> Refused<AgentBlock> {
    let ExprKind::ObjectLiteral(items) = expr else {
        return Err(OutlineRefusal::at(span, "every cast member must be an object"));
    };

    let mut block = AgentBlock::default();
    for item in items {
        let ObjectItem::Entry(entry) = item else {
            return Err(OutlineRefusal::bare("a loop or conditional inside an agent"));
        };
        let Some(key) = entry.static_key() else {
            return Err(OutlineRefusal::at(entry.key.span, "a computed key"));
        };
        let value = &entry.value.node;
        let at = entry.value.span;

        match key {
            "handle" => block.handle = text(value, at)?,
            "name" => block.name = text(value, at)?,
            "kind" => block.kind = text(value, at)?,
            "script" => block.script = text(value, at)?,
            "run_condition" => block.run_condition = text(value, at)?,
            "character_id" => block.character = binding(value, RealmResourceKind::Character, at)?,
            "template_id" => block.template = binding(value, RealmResourceKind::Template, at)?,
            "persona_id" => block.persona = binding(value, RealmResourceKind::Persona, at)?,
            "profile_id" => {
                block.profile = binding(value, RealmResourceKind::ConnectionProfile, at)?
            }
            "lorebooks" => block.lorebooks = bindings(value, RealmResourceKind::Lorebook, at)?,
            // `user_id`, `char_version_id` and the fields the block model does
            // not carry are left alone rather than refused: they are defaults
            // the editor never writes.
            _ => {}
        }
    }

    Ok(block)
}

fn bindings(expr: &ExprKind, kind: RealmResourceKind, span: Span) -> Refused<Vec<Binding>> {
    let ExprKind::ArrayLiteral(items) = expr else {
        return Err(OutlineRefusal::at(span, "`lorebooks` must be an array"));
    };
    items
        .iter()
        .map(|item| match item {
            ArrayItem::Element(expr) => binding(&expr.node, kind, expr.span)?
                .ok_or_else(|| OutlineRefusal::at(expr.span, "a null in `lorebooks`")),
            _ => Err(OutlineRefusal::bare("a loop or conditional in `lorebooks`")),
        })
        .collect()
}

/// One id-shaped field: a literal id, a loop variable, or an input.
fn binding(expr: &ExprKind, kind: RealmResourceKind, span: Span) -> Refused<Option<Binding>> {
    match expr {
        ExprKind::Literal(Value::None) => Ok(None),
        ExprKind::Literal(Value::String(id)) => {
            let id = id.parse::<i64>().map(Snowflake).map_err(|_| {
                OutlineRefusal::at(span, format!("`{id}` is not an id"))
            })?;
            Ok(Some(Binding::Resource { kind, id }))
        }
        // `{{c.id}}` — the loop variable. `{{input:lead.id}}` — the input.
        ExprKind::Variable(VariableRef { scope, name, path }) if reads_id(path) => {
            Ok(Some(match scope.as_deref() {
                None => Binding::Loop {
                    var_name: name.clone(),
                },
                Some("input") => Binding::Input { name: name.clone() },
                Some(other) => {
                    return Err(OutlineRefusal::at(
                        span,
                        format!("`{other}:` in a field the editor fills from a picker"),
                    ));
                }
            }))
        }
        _ => Err(OutlineRefusal::at(
            span,
            "a field the editor fills from a picker holds something else here",
        )),
    }
}

fn reads_id(path: &[PathSegment]) -> bool {
    matches!(path, [PathSegment::Key(key)] if key == "id")
}

/// A string field, back as the author wrote it.
///
/// A raw string is a literal. A quoted one parsed into a template, and the
/// editor wants its source back — `regent_{{c.slug}}`, not the two nodes it
/// became.
fn text(expr: &ExprKind, span: Span) -> Refused<String> {
    match expr {
        ExprKind::Literal(Value::String(s)) => Ok(s.clone()),
        ExprKind::Literal(Value::None) => Ok(String::new()),
        ExprKind::StringTemplate(template) => unparse(template, span),
        _ => Err(OutlineRefusal::at(span, "a text field holds a non-string")),
    }
}

/// Re-render the interpolations the editor emits. Anything richer is refused:
/// the block model has a text box, not an expression editor.
fn unparse(template: &Template, span: Span) -> Refused<String> {
    let mut out = String::new();
    for node in &template.nodes {
        match &node.node {
            NodeKind::Literal(text) => out.push_str(text),
            NodeKind::Expression(ExprKind::Variable(reference)) => {
                out.push_str("{{");
                if let Some(scope) = &reference.scope {
                    out.push_str(scope);
                    out.push(':');
                }
                out.push_str(&reference.name);
                for segment in &reference.path {
                    match segment {
                        PathSegment::Key(key) => {
                            out.push('.');
                            out.push_str(key);
                        }
                        PathSegment::Index(i) => out.push_str(&format!("[{i}]")),
                    }
                }
                out.push_str("}}");
            }
            _ => {
                return Err(OutlineRefusal::at(
                    span,
                    "a string with weaver the editor cannot show as plain text",
                ));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use weaver_lang::parse_value_doc;

    /// A document shaped the way the editor emits one.
    const EDITOR_OUTPUT: &str = r####"
#inputs
regents: [Ref<Character>]
lead: Ref<Character>

{
  agents: [
    {
      char_version_id: null
      character_id: {{input:lead.id}}
      handle: r"narrator"
      kind: r"llm"
      lorebooks: [
        r"40"
      ]
      name: r"The Narrator"
      persona_id: null
      profile_id: r"30"
      run_condition: r"{{self:alive}} == true"
      template_id: r"20"
      user_id: null
    }
    {# foreach c in {{input:regents}} #}
      {
        char_version_id: null
        character_id: {{c.id}}
        handle: "regent_{{c.slug}}"
        kind: r"llm"
        name: "{{c.name}}"
        persona_id: null
        profile_id: null
        template_id: null
        user_id: null
      }
    {# endforeach #}
  ]
  description: r"A realm."
  director: null
  name: r"Sengoku"
  scenarios: []
}
"####;

    fn outlined(source: &str) -> Refused<Outline> {
        outline(&parse_value_doc(source).expect("parses"))
    }

    #[test]
    fn the_editors_own_output_comes_back_as_the_block_model() {
        let got = outlined(EDITOR_OUTPUT).expect("outlines");

        assert_eq!(got.name, "Sengoku");
        assert_eq!(got.description, "A realm.");
        assert_eq!(got.inputs.len(), 2);
        assert_eq!(got.cast.len(), 2);

        let CastEntry::Agent { agent } = &got.cast[0] else {
            panic!("first entry should be a plain agent");
        };
        assert_eq!(agent.handle, "narrator");
        assert_eq!(agent.kind, "llm");
        assert_eq!(agent.run_condition, "{{self:alive}} == true");
        // A singular entity input, read straight off `input:`.
        assert!(matches!(
            agent.character,
            Some(Binding::Input { ref name }) if name == "lead"
        ));
        assert!(matches!(
            agent.template,
            Some(Binding::Resource { id: Snowflake(20), .. })
        ));
        assert!(matches!(
            agent.lorebooks.as_slice(),
            [Binding::Resource { id: Snowflake(40), .. }]
        ));

        let CastEntry::Foreach { var_name, over, body } = &got.cast[1] else {
            panic!("second entry should be a loop");
        };
        assert_eq!(var_name, "c");
        assert_eq!(over.as_deref(), Some("regents"));
        assert_eq!(body.len(), 1);
        // The interpolations survive as source, not as the nodes they parsed to.
        assert_eq!(body[0].handle, "regent_{{c.slug}}");
        assert_eq!(body[0].name, "{{c.name}}");
        assert!(matches!(
            body[0].character,
            Some(Binding::Loop { ref var_name }) if var_name == "c"
        ));
    }

    #[test]
    fn tags_and_a_custom_director_come_back() {
        let got = outlined(
            r####"{
  agents: []
  description: r""
  director: {
    kind: r"script"
    source: r#"{# if {{turn:index}} == 0 #}{# return "narrator" #}{# endif #}"#
  }
  name: r"Sengoku"
  scenarios: []
  tags: [
    r"strategy"
    r"sengoku"
  ]
}"####,
        )
        .expect("outlines");

        assert_eq!(got.tags, ["strategy", "sengoku"]);
        let Some(DirectorBlock::Script { source }) = got.director else {
            panic!("expected a script director, got {:?}", got.director);
        };
        assert!(source.contains("turn:index"), "{source}");
    }

    #[test]
    fn a_filtered_rotation_comes_back() {
        let got = outlined(
            r#"{
  agents: []
  name: r"X"
  director: { kind: r"round_robin", over: { match: r"kind", kind: r"llm" } }
}"#,
        )
        .expect("outlines");

        let Some(DirectorBlock::RoundRobin { over: FilterBlock::Kind { kind } }) = got.director
        else {
            panic!("expected a kind-filtered rotation, got {:?}", got.director);
        };
        assert_eq!(kind, "llm");
    }

    #[test]
    fn a_document_the_editor_cannot_draw_is_refused_by_name() {
        // A conditional in the cast has no block-model equivalent.
        let refusal = outlined(
            r#"{
  name: r"X"
  agents: [
    {# if true #}
    { handle: r"a", name: r"a", kind: r"llm" }
    {# endif #}
  ]
}"#,
        )
        .expect_err("refuses");
        assert!(refusal.message.contains("conditional"), "{refusal:?}");
    }

    #[test]
    fn an_expression_a_text_box_cannot_hold_is_refused() {
        let refusal = outlined(
            r#"{
  name: r"X"
  agents: [
    { handle: "a{{ 1 + 2 }}", name: r"a", kind: r"llm" }
  ]
}"#,
        )
        .expect_err("refuses");
        assert!(refusal.message.contains("plain text"), "{refusal:?}");
    }

    #[test]
    fn a_field_holding_something_other_than_an_id_is_refused_with_its_span() {
        let refusal = outlined(
            r#"{
  name: r"X"
  agents: [
    { handle: r"a", name: r"a", kind: r"llm", character_id: {{global:whatever}} }
  ]
}"#,
        )
        .expect_err("refuses");
        assert!(refusal.from.is_some(), "a refusal should point somewhere");
    }
}
