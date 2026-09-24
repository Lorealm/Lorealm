import json

import wit_world
from wit_world.imports import host
from wit_world.imports.types import (
    Block,
    CommandSignature,
    Event_Command,
    Event_Frontend,
    Event_Processor,
    Event_PromptBuild,
    Event_Source,
    Hook,
    Level,
    Message,
    Param,
    ProcessorSignature,
    Response_Command,
    Response_Frontend,
    Response_Processor,
    Response_PromptBuild,
    Response_Source,
    Role,
    SourceNode,
    ValueType,
)
from componentize_py_types import Err


class WitWorld(wit_world.WitWorld):
    def init(self) -> None:
        host.register_hook(Hook.PROMPT_BUILD)
        host.register_command(
            CommandSignature("greet", "Greet someone by name", ValueType.STRING, [param("name")], False)
        )
        host.register_command(
            CommandSignature(
                "tally",
                "Count one more under `tally:<key>` and return the count",
                ValueType.NUMBER,
                [param("key")],
                True,
            )
        )
        host.register_processor(
            ProcessorSignature(
                "shout", "Uppercase text and add an exclamation mark", ValueType.STRING, [param("text")]
            )
        )
        host.register_source_node(
            SourceNode("greeting", "Greeting", "Greet whoever the node's `name` setting names", None)
        )

    def handle(self, event: wit_world.types.Event) -> wit_world.types.Response:
        if isinstance(event, Event_PromptBuild):
            return Response_PromptBuild(prompt_build(event.value))
        if isinstance(event, Event_Command):
            call = event.value
            arg = string(call.args[0] if call.args else None)
            if call.name == "tally":
                current = call.context.get("tally", arg)
                count = (json.loads(current) if current is not None else 0) + 1
                call.context.set("tally", arg, json.dumps(count))
                return Response_Command(json.dumps(count))
            # Test control: writing from a command that didn't declare it.
            if arg == "sneak":
                call.context.set("tally", "sneak", "1")
                return Response_Command(None)
            return Response_Command(json.dumps(f"Hello, {arg}!"))
        if isinstance(event, Event_Processor):
            text = string(dict(event.value.properties).get("text"))
            return Response_Processor(json.dumps(f"{text.upper()}!"))
        if isinstance(event, Event_Frontend):
            if event.value.name != "greet":
                raise Err(f"unknown frontend call `{event.value.name}`")
            name = json.loads(event.value.args).get("name", "stranger")
            return Response_Frontend(json.dumps(f"Hello, {name}!"))
        if isinstance(event, Event_Source):
            name = json.loads(event.value.settings).get("name", "stranger")
            return Response_Source([Block(Role.SYSTEM, f"Greetings, {name}.")])
        raise Err("unknown event")


def prompt_build(prompt: list[Message]) -> list[Message]:
    # Test controls: the last message's content.
    last = prompt[-1].content if prompt else None
    if last == "loop forever":
        while True:
            pass
    if last == "forget":
        host.delete_data("greetings")
    if last == "crash":
        raise ValueError("crashed on purpose")

    persona = host.active_persona()
    count = greetings() + 1
    host.persist_data("greetings", str(count).encode())
    host.log(Level.INFO, f"greeting #{count}")
    print(f"printed greeting #{count}")
    prompt.append(Message(Role.SYSTEM, f"Hello, {persona.name}! #{count}"))
    return prompt


def greetings() -> int:
    """How many times this user has been greeted."""
    data = host.load_data("greetings")
    if data is None:
        return 0
    try:
        return int(data.decode())
    except ValueError:
        raise Err("corrupt greeting count")


def param(name: str) -> Param:
    return Param(name, "", ValueType.STRING, True)


def string(value: str | None) -> str:
    """A JSON string argument."""
    parsed = json.loads(value) if value is not None else None
    if not isinstance(parsed, str):
        raise Err("expected a string")
    return parsed
