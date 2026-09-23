import wit_world
from componentize_py_types import Err
from wit_world.imports import host
from wit_world.imports.types import Event_PromptBuild, Hook, Level, Message, Role


class WitWorld(wit_world.WitWorld):
    def init(self) -> None:
        host.register_hook(Hook.PROMPT_BUILD)

    def handle(self, event: wit_world.types.Event) -> wit_world.types.Event:
        prompt = event.value
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
        return Event_PromptBuild(prompt)


def greetings() -> int:
    """How many times this user has been greeted."""
    data = host.load_data("greetings")
    if data is None:
        return 0
    try:
        return int(data.decode())
    except ValueError:
        raise Err("corrupt greeting count")
