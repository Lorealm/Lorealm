<template>
  <section class="chat-area">
    <!-- Debug section - remove this once fixed -->
    <div style="background: #333; color: #fff; padding: 10px; margin-bottom: 10px; font-family: monospace;">
      <div>Debug Info:</div>
      <div>templateMessages: {{ templateMessages }}</div>
      <div>templateMessages length: {{ templateMessages?.length }}</div>
      <div>Array.isArray(templateMessages): {{ Array.isArray(templateMessages) }}</div>
      <div>First message: {{ templateMessages?.[0] }}</div>
    </div>
    
    <div class="message-history">
      <div 
        v-for="(message, index) in templateMessages" 
        :key="message.id || index"
        :class="['message', message.role]"
      >
        <div class="message-role">{{ message.role }}</div>
        <div class="message-content">{{ message.content }}</div>
      </div>
    </div>
    <div>
      <div class="extension-area">
        <BotMessageSquare style="min-width: 22px; min-height: 22px;" />
        <strong>Assistant: </strong>
        {{ hypebotManager.getCurrentMessage() || '...' }}
      </div>
      <div class="input-area">
        <textarea
          v-model="templateUserInput"
          placeholder="Type your message here..."
          @keydown="handleKeydown"
          :disabled="isLoading"
        ></textarea>
        <button 
          class="send-btn" 
          @click="sendMessage"
          :disabled="isLoading || !templateUserInput.trim()"
        >
          {{ isLoading ? 'Sending...' : 'Send' }}
        </button>
        <button 
          v-if="isLoading"
          class="cancel-btn" 
          @click="cancelRequest"
        >
          Cancel
        </button>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref, inject, onUnmounted, reactive, computed, nextTick } from 'vue'
import { BotMessageSquare } from 'lucide-vue-next'
import { LLMClient, ConversationManager, HypebotManager} from '@/services/llm-framework'
import { type Message } from '@/types/llm'
import { appStateKey, type AppState } from '@/types/app'

// Inject the app state with proper typing
const app = inject(appStateKey)

console.log('Injected app:', app)
console.log('App messages:', app?.messages)
console.log('App messages value:', app?.messages?.value)

if (!app) {
  console.error('App state not injected! Make sure ChatWindow is inside a component that provides "app"')
}

// Use the injected app state refs directly
const messages = app?.messages || ref([])
const userInput = app?.userInput || ref('')

console.log('Local messages ref:', messages)
console.log('Local messages value:', messages.value)

// Make sure the template can access these properly
const templateMessages = messages
const templateUserInput = userInput

// Initialize LLM framework
const llmClient = new LLMClient({
  baseUrl: '/api',
  defaultSystemPrompt: "Your job is to provide an insightful response to the user's query. Keep your responses under four paragraphs, don't respond to yourself, and don't provide unnecessary information.",
  timeout: 60000,
  retryAttempts: 3
})

// Initialize conversation manager without the problematic sync callback during init
const conversationManager = new ConversationManager(
  llmClient,
  "Your job is to provide an insightful response to the user's query. Keep your responses under four paragraphs, don't respond to yourself, and don't provide unnecessary information."
  // No callback during initialization to prevent clearing
)

// Initialize hypebot manager with streaming
const hypebotManager = reactive(new HypebotManager(
  llmClient,
  (message: string) => {
    // Reactivity is handled by the reactive wrapper
  }
))

// Component state
const isLoading = ref(false)
const currentAbortController = ref<AbortController | null>(null)

// Initialize with existing messages if they exist
if (messages && messages.value && messages.value.length > 0) {
  console.log('Initial messages exist, setting up ConversationManager...')
  
  // Convert and add existing messages to ConversationManager without triggering sync
  const initialMessages: Message[] = messages.value.map((msg: any, index: number) => ({
    ...msg,
    id: msg.id || `initial_${index}`,
    timestamp: msg.timestamp || Date.now()
  }))
  
  // Add messages to conversation manager
  conversationManager.clearMessages()
  for (const message of initialMessages) {
    if (message.role !== 'system') {
      conversationManager.getMessages().push(message)
    }
  }
  
  console.log('ConversationManager initialized with:', conversationManager.getMessages())
}

// NOW set up the sync callback after initialization
const setupSyncCallback = () => {
  // We'll manually sync when new messages are added
  const originalSendMessage = conversationManager.sendMessage.bind(conversationManager)
  
  conversationManager.sendMessage = async (content: string, options: any = {}) => {
    const result = await originalSendMessage(content, {
      ...options,
      onComplete: (response: string) => {
        // Sync after completion
        if (messages && messages.value) {
          const currentMessages = conversationManager.getMessages()
          messages.value.splice(0, messages.value.length, ...currentMessages)
        }
        options.onComplete?.(response)
      }
    })
    return result
  }
}

setupSyncCallback()

async function sendMessage() {
  if (!templateUserInput.value.trim() || isLoading.value) {
    return
  }
  
  const message = templateUserInput.value.trim()
  templateUserInput.value = ''
  isLoading.value = true
  
  // Create abort controller for this request
  currentAbortController.value = new AbortController()
  
  try {
    // Add user message directly to our messages array
    const userMessage: Message = {
      role: 'user',
      content: message,
      id: `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      timestamp: Date.now()
    }
    
    // Create assistant message
    const assistantMessage: Message = {
      role: 'assistant',
      content: '',
      id: `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      timestamp: Date.now()
    }
    
    // Add to our reactive messages array
    messages.value.push(userMessage)
    messages.value.push(assistantMessage)
    
    // Keep track of the assistant message index for updates
    const assistantIndex = messages.value.length - 1
    
    // Use the LLM client directly for streaming
    const response = await llmClient.streamCompletion({
      messages: messages.value.slice(0, -1), // Don't include empty assistant message
      systemPrompt: "Your job is to provide an insightful response to the user's query. Keep your responses under four paragraphs, don't respond to yourself, and don't provide unnecessary information."
    }, {
      onToken: (token) => {
        // Update the message in place and trigger reactivity by reassigning
        assistantMessage.content += token
        // Force reactivity by updating the array reference
        messages.value = [...messages.value]
      },
      onComplete: async (fullResponse) => {
        assistantMessage.content = fullResponse
        
        // Generate hypebot response after main response is complete
        try {
          await hypebotManager.generateResponse(fullResponse, {
            abortController: currentAbortController.value || undefined,
            onError: (error) => {
              console.error('Hypebot error:', error)
            }
          })
        } catch (hypebotError) {
          console.error('Failed to generate hypebot response:', hypebotError)
        }
      },
      onError: (error) => {
        console.error('Error sending message:', error)
        // Remove the failed assistant message
        const messageIndex = messages.value.indexOf(assistantMessage)
        if (messageIndex > -1) {
          messages.value.splice(messageIndex, 1)
        }
      },
      abortController: currentAbortController.value
    })
    
  } catch (error) {
    console.error('Failed to send message:', error)
    if (error instanceof Error && error.message !== 'Request aborted') {
      // Show user-friendly error message
      alert('Failed to send message. Please try again.')
    }
  } finally {
    isLoading.value = false
    currentAbortController.value = null
  }
}

function cancelRequest() {
  if (currentAbortController.value) {
    currentAbortController.value.abort()
    currentAbortController.value = null
  }
  isLoading.value = false
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}

// Cleanup on unmount
onUnmounted(() => {
  llmClient.closeAllConnections()
  if (currentAbortController.value) {
    currentAbortController.value.abort()
  }
})

// Expose methods for external use
defineExpose({
  sendMessage,
  cancelRequest,
  clearMessages: () => conversationManager.clearMessages(),
  getActiveConnections: () => llmClient.getActiveConnectionCount()
})
</script>

<style>
@import url(@/assets/chat-area.css);

.input-area {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}

.send-btn:disabled,
.cancel-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.cancel-btn {
  background: #dc3545;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
}

.cancel-btn:hover {
  background: #c82333;
}
</style>