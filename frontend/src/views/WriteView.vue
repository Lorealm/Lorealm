<template>
  <!-- Write View -->
  <div class="write-view active" id="write-view">
    <!-- Left Panel -->
    <div class="left-panel">
      <div class="character-module">
        <div class="module-header"><span>👤</span> ACTIVE CHARACTER</div>
        <div class="character-portrait">
          <!-- Portrait would go here -->
        </div>
        <div class="character-info">
          <div class="character-name">Morwen the Wise</div>
          <div style="color: #908caa; font-size: 13px; margin: -8px 0 8px 0">
            Scholar-Mage of the Northern Tower
          </div>
          <div class="character-traits">
            <span class="trait-tag">Intelligent</span>
            <span class="trait-tag">Cautious</span>
            <span class="trait-tag">Ancient Knowledge</span>
            <span class="trait-tag">Magic Affinity: High</span>
          </div>
          <div
            style="
              margin-top: 12px;
              padding-top: 12px;
              border-top: 1px solid rgba(138, 127, 219, 0.1);
            "
          >
            <div style="color: #908caa; font-size: 11px; margin-bottom: 8px">CURRENT MOOD</div>
            <select class="select-field" style="width: 100%">
              <option>Curious</option>
              <option>Determined</option>
              <option>Anxious</option>
              <option>Excited</option>
            </select>
          </div>
        </div>
      </div>

      <div class="scene-module">
        <div class="module-header"><span>🎭</span> SCENE CONTEXT</div>
        <div class="scene-details">
          <div class="scene-item">
            <span class="scene-label">Location</span>
            <span class="scene-value">Ancient Library</span>
          </div>
          <div class="scene-item">
            <span class="scene-label">Time</span>
            <span class="scene-value">Midnight</span>
          </div>
          <div class="scene-item">
            <span class="scene-label">Weather</span>
            <span class="scene-value">Stormy</span>
          </div>
          <div class="scene-item">
            <span class="scene-label">Atmosphere</span>
            <span class="scene-value">Mysterious</span>
          </div>
          <div class="scene-item">
            <span class="scene-label">Chapter</span>
            <span class="scene-value">3: The Discovery</span>
          </div>
          <div class="scene-item">
            <span class="scene-label">Word Count</span>
            <span class="scene-value">2,847</span>
          </div>
          <div class="scene-item">
            <span class="scene-label">POV</span>
            <span class="scene-value">Third Person</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Center Editor -->
    <div class="editor-section">
      <div class="editor-controls">
        <div class="story-selector">
          <select class="select-field">
            <option>The Northern Saga</option>
            <option>Desert Chronicles</option>
            <option>New Story</option>
          </select>
          <select class="select-field">
            <option>Chapter 3: The Discovery</option>
            <option>Chapter 2: Journey Begins</option>
            <option>Chapter 1: The Call</option>
          </select>
        </div>
        <div class="editor-actions">
          <button class="btn-action btn-secondary">Fork</button>
          <button class="btn-action btn-primary" onclick="generateResponse()">Generate</button>
        </div>
      </div>

      <div class="chat-container">
        <div class="chat-messages" id="chat-messages">
          <div class="message message-ai">
            <div class="message-content">
              The ancient tome lay before them, its pages yellowed and brittle with age. Morwen
              traced her fingers along the intricate symbols etched into the leather binding,
              feeling the pulse of old magic beneath her touch.
              <br /><br />
              "This is it," she whispered, her voice barely audible in the vast library chamber.
              "The Chronicle of Lost Kingdoms."
            </div>
          </div>
          <div class="message message-user">
            <div class="message-content">
              Morwen opens the tome carefully, searching for any mention of the Shadow Crown. What
              does she find on the first pages?
            </div>
          </div>
          <div class="message message-ai">
            <div class="message-content">
              As the ancient pages part with a soft whisper, dust motes dance in the candlelight
              like tiny spirits awakened from centuries of slumber. The first page bears an
              illuminated map unlike any Morwen has seen – continents that no longer exist, marked
              with kingdoms whose names have been lost to time.
              <br /><br />
              At the center of the map, rendered in fading gold and deep crimson ink, lies an
              illustration of a crown wreathed in darkness. The Shadow Crown. Beneath it, written in
              the old tongue, are words that make Morwen's blood run cold: "Seven were forged in
              shadow's breath, Seven to bind what light forgot."
            </div>
          </div>
        </div>
        <div class="input-container">
          <div class="input-wrapper">
            <textarea class="prompt-input" placeholder="Continue your story..." rows="2"></textarea>
            <button class="btn-action btn-primary" onclick="generateResponse()">Send</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Right Panel -->
    <div class="right-panel">
      <div class="context-widget">
        <div class="module-header"><span>⚡</span> QUICK ACTIONS</div>
        <div class="widget-content">
          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px">
            <button class="btn-action btn-secondary" style="font-size: 11px">Rewrite</button>
            <button class="btn-action btn-secondary" style="font-size: 11px">Continue</button>
            <button class="btn-action btn-secondary" style="font-size: 11px">Summarize</button>
            <button class="btn-action btn-secondary" style="font-size: 11px">Branch</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
.write-view {
  display: none;
  width: 100%;
  padding: 20px;
  gap: 20px;
  grid-template-columns: 340px 1fr 340px;
}

.write-view.active {
  display: grid;
}

/* Left Panel Modules */
.left-panel {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.character-module {
  background: rgba(20, 22, 30, 0.6);
  border: 1px solid rgba(138, 127, 219, 0.1);
  border-radius: 12px;
  padding: 20px;
  backdrop-filter: blur(10px);
  transition: all 0.3s;
}

.character-module:hover {
  border-color: rgba(138, 127, 219, 0.3);
  box-shadow: 0 4px 12px rgba(138, 127, 219, 0.1);
}

.module-header {
  color: #8a7fdb;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.character-portrait {
  width: 100%;
  height: 200px;
  background: linear-gradient(135deg, #8b5cf6 0%, #6366f1 100%);
  border-radius: 8px;
  margin-bottom: 16px;
  position: relative;
  overflow: hidden;
}

.character-portrait img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.character-info {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.character-name {
  font-size: 18px;
  font-weight: 600;
  color: #e0def4;
}

.character-traits {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.trait-tag {
  background: rgba(138, 127, 219, 0.1);
  color: #a5b4fc;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 11px;
  border: 1px solid rgba(138, 127, 219, 0.2);
}

.scene-module {
  background: rgba(20, 22, 30, 0.6);
  border: 1px solid rgba(138, 127, 219, 0.1);
  border-radius: 12px;
  padding: 20px;
  backdrop-filter: blur(10px);
  flex: 1;
}

.scene-details {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.scene-item {
  display: flex;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid rgba(138, 127, 219, 0.05);
}

.scene-label {
  color: #908caa;
  font-size: 12px;
}

.scene-value {
  color: #e0def4;
  font-size: 12px;
  font-weight: 500;
}

/* Center Editor */
.editor-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.editor-controls {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.story-selector {
  display: flex;
  gap: 12px;
}

.select-field {
  background: rgba(20, 22, 30, 0.4);
  border: 1px solid rgba(138, 127, 219, 0.2);
  color: #e0def4;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  outline: none;
  transition: all 0.2s;
}

.select-field:hover,
.select-field:focus {
  border-color: rgba(138, 127, 219, 0.4);
}

.editor-actions {
  display: flex;
  gap: 8px;
}

.btn-action {
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
  border: none;
}

.btn-secondary {
  background: rgba(138, 127, 219, 0.2);
  border: 1px solid rgba(138, 127, 219, 0.3);
  color: #e0def4;
}

.btn-secondary:hover {
  background: rgba(138, 127, 219, 0.3);
}

.btn-primary {
  background: linear-gradient(135deg, #8a7fdb 0%, #6366f1 100%);
  color: white;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(138, 127, 219, 0.3);
}

.chat-container {
  background: rgba(20, 22, 30, 0.4);
  border: 1px solid rgba(138, 127, 219, 0.1);
  border-radius: 12px;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.chat-messages {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.message {
  animation: messageSlide 0.3s ease;
}

@keyframes messageSlide {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.message-user {
  background: rgba(99, 102, 241, 0.1);
  border: 1px solid rgba(99, 102, 241, 0.2);
  padding: 16px;
  border-radius: 8px;
  margin-left: 40px;
}

.message-ai {
  background: rgba(20, 22, 30, 0.4);
  border: 1px solid rgba(138, 127, 219, 0.1);
  padding: 16px;
  border-radius: 8px;
  margin-right: 40px;
}

.message-content {
  color: #e0def4;
  line-height: 1.6;
  font-size: 14px;
}

.input-container {
  padding: 20px;
  border-top: 1px solid rgba(138, 127, 219, 0.1);
  background: rgba(20, 22, 30, 0.2);
}

.input-wrapper {
  display: flex;
  gap: 12px;
}

.prompt-input {
  flex: 1;
  background: rgba(20, 22, 30, 0.6);
  border: 1px solid rgba(138, 127, 219, 0.2);
  color: #e0def4;
  padding: 12px 16px;
  border-radius: 8px;
  font-size: 14px;
  resize: none;
  outline: none;
  transition: all 0.2s;
}

.prompt-input:focus {
  border-color: rgba(138, 127, 219, 0.4);
  background: rgba(20, 22, 30, 0.8);
}

/* Right Panel Modules */
.right-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  padding-right: 4px;
}

.context-widget {
  background: rgba(20, 22, 30, 0.6);
  border: 1px solid rgba(138, 127, 219, 0.1);
  border-radius: 10px;
  padding: 16px;
  backdrop-filter: blur(10px);
  transition: all 0.3s;
}

.context-widget:hover {
  border-color: rgba(138, 127, 219, 0.3);
}

.widget-content {
  margin-top: 12px;
}

.lorebook-entry {
  padding: 8px;
  background: rgba(99, 102, 241, 0.05);
  border-left: 2px solid #6366f1;
  margin-bottom: 8px;
  border-radius: 4px;
  transition: all 0.2s;
  cursor: pointer;
}

.lorebook-entry:hover {
  background: rgba(99, 102, 241, 0.1);
  transform: translateX(2px);
}

.lorebook-title {
  font-weight: 500;
  font-size: 13px;
  color: #e0def4;
}

.lorebook-keys {
  color: #908caa;
  font-size: 11px;
  margin-top: 2px;
}

.weaver-var {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
  font-size: 12px;
}

.var-name {
  color: #a5b4fc;
  font-family: monospace;
}

.var-value {
  color: #e0def4;
}
</style>
