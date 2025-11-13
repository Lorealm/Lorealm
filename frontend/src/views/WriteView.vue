<template>
  <!-- Write View -->
  <div class="write-view active" id="write-view">
    <!-- Left Panel -->
    <div class="left-panel">
      <div class="character-module">
        <div class="panel-header"><span>👤</span> ACTIVE CHARACTER</div>
        <div class="character-portrait">
          <!-- Portrait would go here -->
        </div>
        <div class="character-info">
          <div class="character-name">Morwen the Wise</div>
          <div class="character-subtitle">
            Scholar-Mage of the Northern Tower
          </div>
          <div class="character-traits">
            <span class="trait-tag">Intelligent</span>
            <span class="trait-tag">Cautious</span>
            <span class="trait-tag">Ancient Knowledge</span>
            <span class="trait-tag">Magic Affinity: High</span>
          </div>
          <div class="character-mood-section">
            <div class="character-mood-label">CURRENT MOOD</div>
            <select class="select-field w-full">
              <option>Curious</option>
              <option>Determined</option>
              <option>Anxious</option>
              <option>Excited</option>
            </select>
          </div>
        </div>
      </div>

      <div class="scene-module">
        <div class="panel-header"><span>🎭</span> SCENE CONTEXT</div>
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
          <button class="btn btn-secondary">Fork</button>
          <button class="btn btn-primary" onclick="generateResponse()">Generate</button>
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
            <button class="btn btn-primary" onclick="generateResponse()">Send</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Right Panel -->
    <div class="right-panel">
      <div class="context-widget">
        <div class="panel-header"><span>⚡</span> QUICK ACTIONS</div>
        <div class="widget-content">
          <div class="quick-actions-grid">
            <button class="btn btn-secondary btn-small">Rewrite</button>
            <button class="btn btn-secondary btn-small">Continue</button>
            <button class="btn btn-secondary btn-small">Summarize</button>
            <button class="btn btn-secondary btn-small">Branch</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>