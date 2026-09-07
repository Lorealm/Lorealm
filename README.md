# Lorealm
A self-hostable frontend for LLM chat/roleplay, for the next generation.

## Notice
Lorealm is currently under active development. We have not committed to freezing our APIs yet, so things are bound to break as we continue iterating.

## Introduction
Lorealm (Lore + Realm) is designed to serve the new generation of LLMs. It started off as an opinionated recreation of [SillyTavern](https://github.com/SillyTavern/SillyTavern), but has grown to be so much more.

Lorealm works on the principle of extensibility and control. In practice, this means you have full control over what gets sent to the LLM. The frontend is highly customizable, but if it's not to your liking, you can easily roll your own.

### What makes us different?
Most other tools were created during a time where the future of LLMs was mostly up in the air. Because of this, many features have been bolted on top of an existing codebase. Because Lorealm had the opportunity to be written from scratch, we were able to learn from the shortcomings of other alternatives and roll solutions of our own.

#### The Weaver Language
This is the biggest part of Lorealm, and is available in almost every part of the application. The [Weaver Language](https://github.com/crux-03/weaver_lang) is a small programming language that has been built specifically with Lorealm in mind. It allows us to provide a level of control and complexity ceiling never seen before.

Note that while it is available everywhere, characters, lorebooks, etc. can still be written without ever using it. Weaver raises the ceiling without raising the floor.

Weaver can be used for everything from simple variable subtitution in text, all the way to full-blown procedural game systems.

The Weaver language has a long way to go, but it is already a very powerful tool for those seeking to build unique systems.

#### Reimagined Lorebooks
Lorebooks are ways of providing the model with context about an environment (often a world/universe). Traditional lorebooks systems are pretty simple. They simply look for keywords in your text, then spit out some static (or sometimes semi-dynamic) text. We take it a step further.

Entering [ContextWeaver](https://github.com/crux-03/context_weaver): A modernized lorebook system built on top of the Weaver Language. 

ContextWeaver not only allows you to create lorebooks the traditional way, but also allows you to maintain what we call a "state pool" over the course of a conversation. This allows you to create stateful systems that evolve over time: a friendship tracker, an RPG-like battle system, you name it!

#### First-class tool calling
Tool calling has been a priority from the first lines of code. Nowadays tools are cheap, efficient and incredibly powerful. In Lorealm, a user can create their own tools a model can use by utilizing the Weaver Language. These can be as simple as rolling a dice, or once again, as complex as you'd like. The sky is the limit.

Tools are useful to fill the gaps for what an LLM can't do reliably. Anything revolving randomization is a good example. Tools can also allow the model to interact directly with the state of the world. You'll hear more about it in the next section.

#### Realms
Realms are flagship feature of Lo**realm**. Realms are intended to give you **complete** control over everything, and we really mean *everything*.

A realm can be designed to be as simple or as complex as you'd like. It's essentially a stage for actors. Each actor is given a pool of context, and like everything else, you can adjust exactly what a single agent perceives. This makes realms especially powerful for systems where you don't want omniscient characters. An agent can treat you, the human player, the exact same as any NPC it happens to come across. This is simply not possible with omniscient agents -- you can get close, but the model will always be biased in one way or another.

Realms are currently in an experimental state, and may be hard to grasp. We will continue to improve and simplify it as much as possible as time goes on.

#### Multiple users
This may be of less importance to some, but Lorealm is built around a multi-user architecture. This means you can host your own instances over the network, securely.

## Contributing
We're open to contributions!

We currently don't have many rules, but make sure to read them well:

- We don't accept vibe-coded contributions.
- All contributions should be in English


## License
MIT