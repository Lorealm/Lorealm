wit_bindgen::generate!({
    path: "../../../wit",
    world: "plugin",
});

use lorealm::plugin::host::{
    active_persona, delete_data, emit, http_fetch, load_data, log, persist_data, register_command,
    register_hook, register_processor, register_source_node, settings, subscribe, unsubscribe,
};
use lorealm::plugin::types::{
    Block, CommandCall, CommandSignature, CustomEvent, Finalizing, FrontendCall, Generated, Hook,
    HttpRequest, Level, Message, Param, ProcessorCall, ProcessorSignature, Role, SourceCall,
    SourceNode, UserInput, ValueType, Verdict,
};
use serde_json::Value;

struct Hello;

impl Guest for Hello {
    fn init() -> Result<(), String> {
        register_hook(Hook::PromptBuild)?;
        register_hook(Hook::AfterGenerate)?;
        register_hook(Hook::MessageFinalize)?;
        register_hook(Hook::UserMessage)?;
        // Only `hello_addon` may emit it, as this component also runs as that.
        subscribe("hello_addon.ping")?;
        register_command(&CommandSignature {
            name: "greet".into(),
            description: "Greet someone by name".into(),
            returns: ValueType::String,
            params: vec![param("name")],
            mutates_context: false,
        })?;
        register_command(&CommandSignature {
            name: "tally".into(),
            description: "Count one more under `tally:<key>` and return the count".into(),
            returns: ValueType::Number,
            params: vec![param("key")],
            mutates_context: true,
        })?;
        register_processor(&ProcessorSignature {
            name: "shout".into(),
            description: "Uppercase text and add an exclamation mark".into(),
            returns: ValueType::String,
            properties: vec![param("text")],
        })?;
        register_source_node(&SourceNode {
            id: "greeting".into(),
            title: "Greeting".into(),
            description: "Greet whoever the node's `name` setting names".into(),
            icon: None,
        })
    }

    fn handle(event: Event) -> Result<Response, String> {
        match event {
            Event::PromptBuild(prompt) => prompt_build(prompt).map(Response::PromptBuild),
            Event::Command(CommandCall {
                name,
                args,
                context,
            }) => {
                let arg = string(args.first().map(String::as_str))?;
                match name.as_str() {
                    "tally" => {
                        let count = match context.get("tally", &arg)? {
                            Some(n) => {
                                serde_json::from_str::<u64>(&n).map_err(|e| e.to_string())?
                            }
                            None => 0,
                        } + 1;
                        context.set("tally", &arg, &count.to_string())?;
                        Ok(Response::Command(Some(count.to_string())))
                    }
                    // Test control: writing from a command that didn't declare it.
                    _ if arg == "sneak" => {
                        context.set("tally", "sneak", "1")?;
                        Ok(Response::Command(None))
                    }
                    _ => Ok(Response::Command(Some(json(&format!("Hello, {arg}!"))))),
                }
            }
            Event::Processor(ProcessorCall { properties, .. }) => {
                let text = properties.iter().find(|(key, _)| key == "text");
                let text = string(text.map(|(_, value)| value.as_str()))?;
                Ok(Response::Processor(json(&format!(
                    "{}!",
                    text.to_uppercase()
                ))))
            }
            Event::Frontend(FrontendCall { name, args }) => match name.as_str() {
                "greet" => {
                    let args: Value = serde_json::from_str(&args).map_err(|e| e.to_string())?;
                    let name = args["name"].as_str().unwrap_or("stranger");
                    Ok(Response::Frontend(json(&format!("Hello, {name}!"))))
                }
                "settings" => Ok(Response::Frontend(settings()?)),
                "emit" => {
                    let args: Value = serde_json::from_str(&args).map_err(|e| e.to_string())?;
                    emit(
                        args["topic"].as_str().unwrap_or_default(),
                        &args["payload"].to_string(),
                    )?;
                    Ok(Response::Frontend("null".into()))
                }
                "subscribe" => {
                    subscribe(string(Some(&args))?.as_str())?;
                    Ok(Response::Frontend("null".into()))
                }
                "unsubscribe" => {
                    unsubscribe(string(Some(&args))?.as_str())?;
                    Ok(Response::Frontend("null".into()))
                }
                // The last event received, as `topic payload` (or `after-generate reply`).
                "event" => {
                    let event =
                        load_data("event")?.map(|b| String::from_utf8_lossy(&b).into_owned());
                    Ok(Response::Frontend(Value::from(event).to_string()))
                }
                // Answers with the response body as text.
                "fetch" => {
                    let args: Value = serde_json::from_str(&args).map_err(|e| e.to_string())?;
                    let headers = args["headers"]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                        .collect();
                    let response = http_fetch(&HttpRequest {
                        method: "GET".into(),
                        url: args["url"].as_str().unwrap_or_default().into(),
                        headers,
                        body: None,
                    })?;
                    Ok(Response::Frontend(json(&String::from_utf8_lossy(
                        &response.body,
                    ))))
                }
                other => Err(format!("unknown frontend call `{other}`")),
            },
            Event::Custom(CustomEvent { topic, payload }) => {
                persist_data("event", format!("{topic} {payload}").as_bytes())?;
                // Test control: sends it on again, forever.
                if payload == json("echo") {
                    emit(&topic, &payload)?;
                }
                Ok(Response::Custom)
            }
            Event::AfterGenerate(Generated { content, .. }) => {
                persist_data("event", format!("after-generate {content}").as_bytes())?;
                Ok(Response::AfterGenerate)
            }
            Event::MessageFinalize(Finalizing { content, .. }) => {
                Ok(Response::MessageFinalize(format!("{content}!")))
            }
            // Test control: refuses `deny me`.
            Event::UserMessage(UserInput { content, .. }) => {
                Ok(Response::UserMessage(match content.as_str() {
                    "deny me" => Verdict::Deny("denied on purpose".into()),
                    _ => Verdict::Change(format!("{content}?")),
                }))
            }
            Event::Source(SourceCall { settings, .. }) => {
                let settings: Value = serde_json::from_str(&settings).map_err(|e| e.to_string())?;
                let name = settings["name"].as_str().unwrap_or("stranger");
                Ok(Response::Source(vec![Block {
                    role: Role::System,
                    content: format!("Greetings, {name}."),
                }]))
            }
        }
    }
}

fn prompt_build(mut prompt: Vec<Message>) -> Result<Vec<Message>, String> {
    // Test controls: the last message's content.
    match prompt.last().map(|m| m.content.as_str()) {
        Some("loop forever") => loop {},
        Some("forget") => delete_data("greetings")?,
        Some("crash") => panic!("crashed on purpose"),
        _ => {}
    }
    let persona = active_persona()?;
    let count = greetings()? + 1;
    persist_data("greetings", count.to_string().as_bytes())?;
    log(Level::Info, &format!("greeting #{count}"));
    println!("printed greeting #{count}");
    prompt.push(Message {
        role: Role::System,
        content: format!("Hello, {}! #{count}", persona.name),
    });
    Ok(prompt)
}

/// How many times this user has been greeted.
fn greetings() -> Result<u32, String> {
    let Some(bytes) = load_data("greetings")? else {
        return Ok(0);
    };
    String::from_utf8(bytes)
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "corrupt greeting count".to_owned())
}

fn param(name: &str) -> Param {
    Param {
        name: name.into(),
        description: String::new(),
        value_type: Some(ValueType::String),
        required: true,
    }
}

/// A JSON string argument, or an error naming what was wrong with it.
fn string(value: Option<&str>) -> Result<String, String> {
    match value.map(serde_json::from_str::<Value>) {
        Some(Ok(Value::String(s))) => Ok(s),
        _ => Err("expected a string".into()),
    }
}

fn json(text: &str) -> String {
    Value::String(text.into()).to_string()
}

export!(Hello);
