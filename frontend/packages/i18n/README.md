# @lorealm/i18n

Interface text for Lorealm, in [ICU MessageFormat](https://unicode-org.github.io/icu/userguide/format_parse/messages/).

- `src/locales/en/*.json` is the English source. Edit text here.
- `src/locales/<code>/*.json` are translations. Crowdin writes these; don't edit them by hand.
- `src/generated/` is written by `bun run generate`. Don't edit it.

## Adding or changing a message

1. Edit the English JSON.
2. Run `bun run generate` to update the `Messages` type.
3. Use it as `t.<file>.<key>`. A message with arguments is a function that takes them by name.

```json
{
	"stopCommand": "Stop /{name}",
	"entries": "{count, plural, one {# entry} other {# entries}}",
	"holds": "Holds {list}{more, select, true {, and more} other {}}."
}
```

```ts
t.chat.composer.stopCommand({ name });
t.characters.lorebookImport.entries({ count: 3 }); // "3 entries"
```

Argument types come from the English message: `{x, number}` and `{x, plural, …}` take a number,
a `select` with only `true`/`other` takes a boolean, and a plain `{x}` takes a string.

Write whole sentences. Translators can reorder `{subject} with {entries}`, but they can't fix a sentence
that the code builds out of separate messages.

To show `{`, `}` or `#` literally, wrap them in apostrophes: `'{{'name'}}'` shows `{{name}}`.

## Checks

`bun run check` fails when `src/generated/` is out of date, when a translation doesn't parse, or when it
uses an argument the English message doesn't have. A translation that's missing or broken at runtime
falls back to English.
