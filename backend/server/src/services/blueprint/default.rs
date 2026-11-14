/// A hand authored blueprint to mirror SillyTavern's group chats
pub static GROUP_CHAT_BP: &str = r##"#inputs
chars: [{char: Ref<Character>, talkativeness: range(0, 1)}]
user_persona: Ref<Persona>
user_connection: Ref<ConnectionProfile>
user_template: Ref<Template>
char_connection: Ref<ConnectionProfile>
char_template: Ref<Template>
scenario: {title: string, content: string}

{
  name: r"Group Chat"
  description: r"Create a group chat with you and a list of selected characters."
  introduction: r"# Lorealm Group Chat
  This is the default template shipped with Lorealm and shows how to use Realms to create a simple system.

  It features a custom director script and a range of input values. Play around with it!"
  tags: [
    r"Utility"
  ]
  director: {
    kind: r"script"
    source: r#"{# foreach p in {{cast:player.members}} #}
  {# if {{turn:index}} == 0 #}
    {# return p.handle #}
  {# endif #}
  {# if {{turn:last.kind}} != "player" #}
    {# return p.handle #}
  {# endif #}
{# endforeach #}

// `cast:llm.members` is one object per LLM-typed agent, carrying that agent's
// properties. The Realm system seeds it every turn.
// Members with zero talkativeness are never picked; `none` when nobody can speak.
// The `local` namespace is cleared every turn.
$[set_var(
  "local:next",
  @[random.weighted(items: {{cast:llm.members}}, key: "talkativeness")]
)]

{# if {{local:next}} #}
  {# return {{local:next.handle}} #}
{# endif #}
{# return null #}"#
  }
  scenarios: [
    {# if "{{input:scenario.title}}" != r"" #}
      {
        name: "{{input:scenario.title}}"
        description: r"A custom scenario."
        opening: "{{input:scenario.content}}"
      }
    {# endif #}
  ]
  config: {}
  agents: [
    {
      char_version_id: null
      character_id: null
      handle: "{{input:user_persona.slug}}"
      kind: r"player"
      name: "{{input:user_persona.name}}"
      persona_id: {{input:user_persona.id}}
      profile_id: {{input:user_connection.id}}
      template_id: {{input:user_template.id}}
      user_id: null
    }
    {# foreach c in {{input:chars}} #}
      {
        char_version_id: null
        character_id: {{c.char.id}}
        handle: "{{c.char.slug}}"
        kind: r"llm"
        name: "{{c.char.name}}"
        persona_id: null
        profile_id: {{input:char_connection.id}}
        properties: {
          talkativeness: {{c.talkativeness}}
        }
        template_id: {{input:char_template.id}}
        user_id: null
      }
    {# endforeach #}
  ]
}
"##;
