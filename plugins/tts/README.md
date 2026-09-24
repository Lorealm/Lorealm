# Text to Speech

Adds a **Listen** button under the latest reply, and reads new replies aloud as they finish.

## Setup

1. Add an API key under Connections → API keys: an OpenRouter key, or an ElevenLabs one (listed under "From plugins").
2. Under Settings → You → Plugins → Text to Speech, pick the provider and your key.

## Settings

- **provider**: `openrouter` or `elevenlabs`.
- **autoplay**: read each new reply aloud as soon as it finishes.
- **openrouter_model**, **openrouter_voice**: any text-to-speech model on OpenRouter, and one of its voices.
- **elevenlabs_model**, **elevenlabs_voice**: an ElevenLabs model, and a voice id from your voice library.

Formatting like `*emphasis*`, headings and link addresses isn't read out.
