use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use weaver_lang::{
    CompiledExpr, CompiledTemplate, InputDecl, InputType, Registry, Span,
    ast::{
        ArrayItem, CallArg, Expr, ExprKind, Node, NodeKind, ObjectItem, Template, ValueForEach,
        ValueIf,
    },
    parse_value_doc,
};

use crate::services::weaver::{WeaverHost, WeaverProfile};
use crate::util::weaver::summarize_parser_text;

/// Which compiler the source belongs to.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dialect {
    /// Text with weaver embedded in it.
    #[default]
    Template,
    /// A single value, the way a run condition is.
    Expression,
    /// A WTN document: one value with holes, plus its `#inputs`.
    Document,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// One problem, positioned for a text editor.
///
/// `from`/`to` are offsets in UTF-16 code units. Weaver spans
/// are byte offsets.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub from: usize,
    pub to: usize,
    pub severity: Severity,
    pub message: String,
    pub hint: Option<String>,
}

/// What the source is checked against.
fn known_host(profile: Option<WeaverProfile>) -> WeaverHost {
    let builder = WeaverHost::builder();
    match profile {
        Some(profile) => builder.declarations_for(profile).build(),
        None => builder.declarations_only().build(),
    }
}

fn host_commands() -> HashSet<String> {
    context_weaver::host_command_signatures()
        .into_iter()
        .map(|sig| sig.name)
        .collect()
}

pub fn check(source: &str, dialect: Dialect, profile: Option<WeaverProfile>) -> Vec<Diagnostic> {
    if source.trim().is_empty() {
        return Vec::new();
    }

    let host = known_host(profile);
    let mut checker = Checker {
        source,
        registry: host.registry(),
        host_commands: host_commands(),
        profile,
        out: Vec::new(),
    };

    match dialect {
        Dialect::Template => match CompiledTemplate::compile(source) {
            Ok(compiled) => checker.template(compiled.ast()),
            Err(errors) => return checker.parse_errors(errors),
        },
        Dialect::Expression => match CompiledExpr::compile(source) {
            Ok(compiled) => {
                checker.expr(compiled.ast());
                checker.trailing(compiled.ast().span.end);
            }
            Err(errors) => return checker.parse_errors(errors),
        },
        Dialect::Document => match parse_value_doc(source) {
            Ok(doc) => {
                checker.declarations(&doc.inputs);
                checker.expr(&doc.value);
            }
            Err(errors) => return checker.parse_errors(errors),
        },
    }

    checker.out
}

struct Checker<'a> {
    source: &'a str,
    registry: &'a Registry,
    host_commands: HashSet<String>,
    profile: Option<WeaverProfile>,
    out: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn parse_errors(&self, errors: Vec<weaver_lang::ParseError>) -> Vec<Diagnostic> {
        errors
            .into_iter()
            .map(|error| Diagnostic {
                from: utf16_offset(self.source, error.span.start),
                to: utf16_offset(self.source, error.span.end),
                severity: Severity::Error,
                message: summarize_parser_text(&error.message),
                hint: error.hint.as_deref().map(summarize_parser_text),
            })
            .collect()
    }

    fn error(&mut self, span: Span, message: String, hint: Option<&str>) {
        self.out.push(Diagnostic {
            from: utf16_offset(self.source, span.start),
            to: utf16_offset(self.source, span.end),
            severity: Severity::Error,
            message,
            hint: hint.map(str::to_string),
        });
    }

    fn declarations(&mut self, declarations: &[InputDecl]) {
        for declaration in declarations {
            let mut kinds = Vec::new();
            entity_kinds(&declaration.ty, &mut kinds);

            let mut unknown = false;
            for kind in &kinds {
                if !self.registry.has_kind(kind) {
                    self.error(
                        declaration.span,
                        format!("no such entity kind: {kind}"),
                        Some("Ref names a kind the server resolves, like Ref<Character>"),
                    );
                    unknown = true;
                }
            }
            if unknown {
                continue;
            }

            if !kinds.is_empty() && !declaration.is_required() {
                self.error(
                    declaration.span,
                    format!(
                        "input '{}' names an entity, so it cannot have a default",
                        declaration.name
                    ),
                    Some(
                        "entity inputs are resolved before the document runs, and a default has not been evaluated by then",
                    ),
                );
            }
        }
    }

    fn warn(&mut self, span: Span, message: String) {
        self.out.push(Diagnostic {
            from: utf16_offset(self.source, span.start),
            to: utf16_offset(self.source, span.end),
            severity: Severity::Warning,
            message,
            hint: None,
        });
    }

    fn warn_with_hint(&mut self, span: Span, message: String, hint: String) {
        self.out.push(Diagnostic {
            from: utf16_offset(self.source, span.start),
            to: utf16_offset(self.source, span.end),
            severity: Severity::Warning,
            message,
            hint: Some(hint),
        });
    }

    fn variable(&mut self, var: &weaver_lang::ast::VariableRef, span: Span) {
        let Some(profile) = self.profile else {
            return;
        };
        if var.scope.as_deref() != Some("input") || profile == WeaverProfile::RealmTemplate {
            return;
        }
        self.warn_with_hint(
            span,
            format!("`input:{}` is empty outside a WTN document", var.name),
            "Inputs are only available during blueprint expansion. If you want to access these\
            values during runtime evaluation, bind them to a place they persist, such as an agent's properties."
                .to_string(),
        );
    }

    /// Text left over after the expression, reported where it starts.
    fn trailing(&mut self, end: usize) {
        let rest = &self.source[end..];
        let leading_ws = rest.len() - rest.trim_start().len();
        let rest = rest.trim();
        if rest.is_empty() {
            return;
        }
        let start = end + leading_ws;
        self.out.push(Diagnostic {
            from: utf16_offset(self.source, start),
            to: utf16_offset(self.source, start + rest.len()),
            severity: Severity::Error,
            message: "unexpected text after the expression".to_string(),
            hint: Some("an expression is a single value".to_string()),
        });
    }

    fn template(&mut self, template: &Template) {
        for node in &template.nodes {
            self.node(node);
        }
    }

    fn node(&mut self, node: &Node) {
        match &node.node {
            NodeKind::Literal(_) | NodeKind::Break | NodeKind::Continue | NodeKind::Stop => {}
            NodeKind::Expression(kind) => self.expr_kind(kind, node.span),
            NodeKind::Command(call) => {
                self.command(&call.name, node.span);
                self.args(&call.args);
            }
            NodeKind::IfBlock(block) => {
                self.expr(&block.condition);
                self.template(&block.body);
                for branch in &block.elif_branches {
                    self.expr(&branch.condition);
                    self.template(&branch.body);
                }
                if let Some(body) = &block.else_body {
                    self.template(body);
                }
            }
            NodeKind::ForEach(block) => {
                self.expr(&block.iterable);
                self.template(&block.body);
            }
            NodeKind::Return(Some(expr)) => self.expr(expr),
            NodeKind::Return(None) => {}
        }
    }

    fn expr(&mut self, expr: &Expr) {
        self.expr_kind(&expr.node, expr.span);
    }

    fn args(&mut self, args: &[CallArg]) {
        for arg in args {
            self.expr(&arg.value);
        }
    }

    /// A loop standing in for items of an array or object literal.
    fn value_foreach<T>(&mut self, block: &ValueForEach<T>, item: fn(&mut Self, &T)) {
        self.expr(&block.iterable);
        for entry in &block.body {
            item(self, entry);
        }
    }

    /// A conditional standing in for items of an array or object literal.
    fn value_if<T>(&mut self, block: &ValueIf<T>, item: fn(&mut Self, &T)) {
        for (condition, body) in &block.branches {
            self.expr(condition);
            for entry in body {
                item(self, entry);
            }
        }
        for entry in block.else_body.iter().flatten() {
            item(self, entry);
        }
    }

    fn array_item(&mut self, item: &ArrayItem) {
        match item {
            ArrayItem::Element(expr) => self.expr(expr),
            ArrayItem::ForEach(block) => self.value_foreach(block, Self::array_item),
            ArrayItem::If(block) => self.value_if(block, Self::array_item),
        }
    }

    fn object_item(&mut self, item: &ObjectItem) {
        match item {
            ObjectItem::Entry(entry) => {
                self.expr(&entry.key);
                self.expr(&entry.value);
            }
            ObjectItem::ForEach(block) => self.value_foreach(block, Self::object_item),
            ObjectItem::If(block) => self.value_if(block, Self::object_item),
        }
    }

    fn expr_kind(&mut self, kind: &ExprKind, span: Span) {
        match kind {
            ExprKind::Literal(_) => {}
            ExprKind::Variable(var) => self.variable(var, span),
            ExprKind::ArrayLiteral(items) => {
                for item in items {
                    self.array_item(item);
                }
            }
            ExprKind::ObjectLiteral(items) => {
                for item in items {
                    self.object_item(item);
                }
            }
            ExprKind::StringTemplate(template) => self.template(template),
            ExprKind::Variant { values, .. } => {
                for value in values {
                    self.expr(value);
                }
            }
            ExprKind::ProcessorCall(call) => {
                if self
                    .registry
                    .processor_signature(&call.namespace, &call.name)
                    .is_none()
                {
                    let name = format!("{}.{}", call.namespace, call.name);
                    self.warn(span, format!("unknown processor `{name}`"));
                }
                self.args(&call.args);
            }
            ExprKind::CommandCall(call) => {
                self.command(&call.name, span);
                self.args(&call.args);
            }
            ExprKind::Trigger(trigger) => self.expr(&trigger.entry_id),
            ExprKind::Document(document) => self.expr(&document.document_id),
            ExprKind::BinaryOp { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::UnaryOp { operand, .. } => self.expr(operand),
            ExprKind::Index { base, index } => {
                self.expr(base);
                self.expr(index);
            }
        }
    }

    fn command(&mut self, name: &str, span: Span) {
        if self.registry.command_signature(name).is_none() && !self.host_commands.contains(name) {
            self.warn(span, format!("unknown command `{name}`"));
        }
    }
}

/// Every entity kind a declaration names.
fn entity_kinds<'a>(ty: &'a InputType, out: &mut Vec<&'a str>) {
    match ty {
        InputType::Ref(kind) => out.push(kind),
        InputType::List(inner) => entity_kinds(inner, out),
        InputType::Object(fields) => {
            for (_, ty) in fields {
                entity_kinds(ty, out);
            }
        }
        _ => {}
    }
}

fn utf16_offset(source: &str, byte: usize) -> usize {
    let mut byte = byte.min(source.len());
    while !source.is_char_boundary(byte) {
        byte -= 1;
    }
    source[..byte].encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(source: &str) -> Vec<Diagnostic> {
        check(source, Dialect::Template, None)
    }

    fn expression(source: &str) -> Vec<Diagnostic> {
        check(source, Dialect::Expression, None)
    }

    /// A document, checked the way a blueprint is.
    fn document(source: &str) -> Vec<Diagnostic> {
        check(
            source,
            Dialect::Document,
            Some(WeaverProfile::RealmTemplate),
        )
    }

    fn messages(found: &[Diagnostic]) -> Vec<&str> {
        found.iter().map(|d| d.message.as_str()).collect()
    }

    #[test]
    fn a_document_is_checked_as_a_document_not_as_text() {
        let source = r#"
#inputs
regents: [Ref<Wizard>]

{
  name: "Sengoku"
  agents: [
    {# foreach c in {{input:regents}} #}
    { handle: "regent_{{c.slug}}" }
    {# endforeach #}
  ]
}"#;
        assert!(template(source).is_empty(), "text mode sees nothing here");

        let found = document(source);
        assert_eq!(messages(&found), ["no such entity kind: Wizard"]);
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(marked(source, &found[0]), "regents: [Ref<Wizard>]");
    }

    #[test]
    fn every_registered_kind_is_accepted_and_others_are_not() {
        for kind in lorealm_storage::models::RealmResourceKind::ALL {
            let source = format!("#inputs\nx: Ref<{}>\n\n{{}}", kind.ref_name());
            assert!(document(&source).is_empty(), "{}", kind.ref_name());
        }
        assert_eq!(
            messages(&document("#inputs\nx: Ref<Realm>\n\n{}")),
            ["no such entity kind: Realm"]
        );
    }

    #[test]
    fn an_entity_input_with_a_default_is_underlined_where_it_is_declared() {
        let source = "#inputs\nlead: Ref<Character> = \"12345\"\n\n{}";
        let found = document(source);

        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].message.contains("cannot have a default"),
            "{}",
            found[0].message
        );
        assert_eq!(
            marked(source, &found[0]),
            "lead: Ref<Character> = \"12345\""
        );
    }

    #[test]
    fn a_plain_input_may_carry_a_default() {
        assert!(document("#inputs\nrounds: number = 3\n\n{}").is_empty());
        assert!(document("#inputs\nmode: enum(\"a\", \"b\") = \"a\"\n\n{}").is_empty());
    }

    #[test]
    fn a_document_that_does_not_parse_reports_where() {
        let found = document("{ name: }");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].severity, Severity::Error);
    }

    #[test]
    fn a_profile_narrows_the_vocabulary_to_its_own_host() {
        let body = "$[toast(\"hi\")]";
        assert!(
            check(body, Dialect::Template, None).is_empty(),
            "the union accepts it"
        );

        let found = check(body, Dialect::Template, Some(WeaverProfile::RealmScript));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("toast"), "{}", found[0].message);

        // And a command the host does carry stays clean.
        assert!(
            check(
                "$[emit_event(\"main\", \"hi\")]",
                Dialect::Template,
                Some(WeaverProfile::RealmScript)
            )
            .is_empty()
        );
    }

    fn marked(source: &str, diagnostic: &Diagnostic) -> String {
        let units: Vec<u16> = source.encode_utf16().collect();
        String::from_utf16(&units[diagnostic.from..diagnostic.to]).unwrap()
    }

    #[test]
    fn clean_source_reports_nothing() {
        assert!(template("Hello, {{global:name}}! $[set_var(\"global:x\", 1)]").is_empty());
        assert!(template("{# foreach x in {{global:items}} #}{{x}}{# endforeach #}").is_empty());
        assert!(expression("{{global:hp}} > 5").is_empty());
    }

    #[test]
    fn the_output_commands_resolve_rather_than_underlining_as_unknown() {
        assert!(template(r#"$[sys("done")]"#).is_empty());
        assert!(template(r#"$[toast("done", "success")]"#).is_empty());
        assert!(template(r#"$[generate_raw("what happened?")]"#).is_empty());
        assert!(template(r#"$[generate("what happened?")]"#).is_empty());
    }

    #[test]
    fn blank_source_is_not_a_mistake() {
        assert!(template("").is_empty());
        assert!(expression("   \n  ").is_empty());
    }

    #[test]
    fn a_syntax_error_is_an_error_not_a_warning() {
        let found = template("{# if {{global:hp}} #}unclosed");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, Severity::Error);
    }

    #[test]
    fn break_outside_a_loop_is_reported_where_it_is_written() {
        let source = "before {# break #} after";
        let found = template(source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(marked(source, &found[0]), "{# break #}");
    }

    #[test]
    fn diagnostics_carry_a_sentence_not_a_source_drawing() {
        let found = template("{{");
        assert_eq!(found.len(), 1);
        assert!(!found[0].message.contains('\n'), "{}", found[0].message);
        assert!(!found[0].message.contains("-->"), "{}", found[0].message);

        // Hints come from weaver and get the same treatment.
        let found = template("{# break #}");
        let hint = found[0].hint.as_deref().unwrap();
        assert!(!hint.contains("  "), "{hint}");
    }

    #[test]
    fn break_inside_a_loop_is_fine() {
        assert!(
            template("{# foreach x in {{global:items}} #}{# break #}{# endforeach #}").is_empty()
        );
    }

    #[test]
    fn every_misplaced_flow_statement_is_reported_at_once() {
        let found = template("{# break #}{# continue #}");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn unknown_callables_are_warnings_pointing_at_the_call() {
        let source = "{{global:a}} $[set_va(\"x\")] and @[math.addd(a: 1)]";
        let found = template(source);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|d| d.severity == Severity::Warning));
        assert_eq!(marked(source, &found[0]), "$[set_va(\"x\")]");
        assert_eq!(found[0].message, "unknown command `set_va`");
        assert_eq!(marked(source, &found[1]), "@[math.addd(a: 1)]");
        assert_eq!(found[1].message, "unknown processor `math.addd`");
    }

    #[test]
    fn callables_registered_per_host_are_still_known() {
        assert!(template("$[is_active(\"entry\")]").is_empty());
        assert!(template("$[emit_event(\"scope\", \"content\")]").is_empty());
        assert!(template("$[attach(\"widget\")]").is_empty());
        assert!(template("@[random.gen(min: 1, max: 6)]").is_empty());
    }

    #[test]
    fn calls_nested_in_expressions_are_checked_too() {
        let found = template("{# if @[math.nope(a: 1)] #}x{# endif #}");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].message, "unknown processor `math.nope`");

        let found = template("{# foreach x in [$[nope()], 2] #}{{x}}{# endforeach #}");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].message, "unknown command `nope`");
    }

    #[test]
    fn a_condition_is_checked_as_an_expression() {
        assert!(template("just prose").is_empty());
        assert_eq!(expression("just prose").len(), 1);

        let found = expression("$[set_va(\"x\")]");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].message, "unknown command `set_va`");
    }

    #[test]
    fn text_past_the_end_of_an_expression_is_reported_not_dropped() {
        let source = "alice is here";
        let found = expression(source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(marked(source, &found[0]), "is here");

        assert!(expression("{{global:hp}} > 5  ").is_empty());
    }

    #[test]
    fn offsets_count_utf16_units_not_bytes() {
        // "🜁 café" is 10 bytes before the call but 7 UTF-16 units, so a
        // byte offset would mark the wrong span in an editor.
        let source = "🜁 café $[set_va(\"x\")]";
        let found = template(source);
        assert_eq!(found.len(), 1);
        assert_eq!(marked(source, &found[0]), "$[set_va(\"x\")]");
        assert_ne!(found[0].from, 10, "byte offset leaked into the response");
    }

    #[test]
    fn a_variable_is_never_flagged() {
        assert!(template("{{global:nothing}} {{bare.path.here}}").is_empty());
    }

    #[test]
    fn inputs_are_flagged_outside_the_document_that_binds_them() {
        let source = r#"{# if {{input:scenario.title}} != "" #}yes{# endif #}"#;

        let in_script = check(source, Dialect::Template, Some(WeaverProfile::RealmScript));
        assert_eq!(in_script.len(), 1);
        assert_eq!(in_script[0].severity, Severity::Warning);
        assert_eq!(marked(source, &in_script[0]), "{{input:scenario.title}}");
        assert_eq!(
            in_script[0].message,
            "`input:scenario` is empty outside a WTN document"
        );

        for profile in [WeaverProfile::RealmDirector, WeaverProfile::RealmCondition] {
            assert_eq!(
                check(source, Dialect::Template, Some(profile)).len(),
                1,
                "{profile:?} does not bind inputs either"
            );
        }

        assert!(
            check(
                source,
                Dialect::Template,
                Some(WeaverProfile::RealmTemplate)
            )
            .is_empty()
        );
        assert!(template(source).is_empty());
        assert!(
            check(
                "{{cast:count}} {{self:gold}} {{nation:a.gold}}",
                Dialect::Template,
                Some(WeaverProfile::RealmScript)
            )
            .is_empty()
        );
    }
}
