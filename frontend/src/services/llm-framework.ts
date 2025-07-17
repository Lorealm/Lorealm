import type { CompletionRequest, LLMClientConfig, Message, StreamingOptions } from "@/types/llm"

export class LLMClient {
  private baseUrl: string
  private defaultSystemPrompt: string
  private timeout: number
  private retryAttempts: number
  private retryDelay: number
  private activeConnections = new Map<string, EventSource>()

  constructor(config: LLMClientConfig = {}) {
    this.baseUrl = config.baseUrl || '/api'
    this.defaultSystemPrompt = config.defaultSystemPrompt || 'You are a helpful assistant.'
    this.timeout = config.timeout || 30000
    this.retryAttempts = config.retryAttempts || 3
    this.retryDelay = config.retryDelay || 1000
  }

  /**
   * Send a streaming completion request
   */
  async streamCompletion(
    request: CompletionRequest,
    options: StreamingOptions = {}
  ): Promise<string> {
    const { onToken, onComplete, onError, abortController } = options
    const connectionId = this.generateConnectionId()
    
    return new Promise((resolve, reject) => {
      let fullResponse = ''
      
      try {
        const messagesPayload = this.prepareMessages(request)
        const streamUrl = `${this.baseUrl}/stream?messages=${encodeURIComponent(JSON.stringify(messagesPayload))}`
        
        const eventSource = new EventSource(streamUrl)
        this.activeConnections.set(connectionId, eventSource)
        
        // Handle abort signal
        if (abortController) {
          abortController.signal.addEventListener('abort', () => {
            this.closeConnection(connectionId)
            reject(new Error('Request aborted'))
          })
        }
        
        // Set timeout
        const timeoutId = setTimeout(() => {
          this.closeConnection(connectionId)
          reject(new Error('Request timeout'))
        }, this.timeout)
        
        eventSource.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data)
            
            if (data.text) {
              fullResponse += data.text
              onToken?.(data.text)
            }
            
            if (data.done) {
              clearTimeout(timeoutId)
              this.closeConnection(connectionId)
              onComplete?.(fullResponse)
              resolve(fullResponse)
            }
          } catch (parseError) {
            clearTimeout(timeoutId)
            this.closeConnection(connectionId)
            const error = new Error(`Failed to parse streaming response: ${parseError}`)
            onError?.(error)
            reject(error)
          }
        }
        
        eventSource.onerror = (event) => {
          clearTimeout(timeoutId)
          this.closeConnection(connectionId)
          const error = new Error('Streaming connection error')
          onError?.(error)
          reject(error)
        }
        
      } catch (error) {
        const err = error instanceof Error ? error : new Error(String(error))
        onError?.(err)
        reject(err)
      }
    })
  }

  /**
   * Send a non-streaming completion request
   */
  async completion(request: CompletionRequest): Promise<string> {
    const messagesPayload = this.prepareMessages(request)
    
    for (let attempt = 1; attempt <= this.retryAttempts; attempt++) {
      try {
        const response = await fetch(`${this.baseUrl}/completion`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            messages: messagesPayload,
            temperature: request.temperature,
            maxTokens: request.maxTokens,
          }),
        })
        
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`)
        }
        
        const data = await response.json()
        return data.text || data.content || ''
        
      } catch (error) {
        if (attempt === this.retryAttempts) {
          throw error
        }
        
        await this.delay(this.retryDelay * attempt)
      }
    }
    
    throw new Error('Max retry attempts exceeded')
  }

  /**
   * Close a specific connection
   */
  closeConnection(connectionId: string): void {
    const connection = this.activeConnections.get(connectionId)
    if (connection) {
      connection.close()
      this.activeConnections.delete(connectionId)
    }
  }

  /**
   * Close all active connections
   */
  closeAllConnections(): void {
    for (const [id, connection] of this.activeConnections) {
      connection.close()
    }
    this.activeConnections.clear()
  }

  /**
   * Get the number of active connections
   */
  getActiveConnectionCount(): number {
    return this.activeConnections.size
  }

  private prepareMessages(request: CompletionRequest): Message[] {
    const systemPrompt = request.systemPrompt || this.defaultSystemPrompt
    
    return [
      {
        role: 'system',
        content: systemPrompt,
        timestamp: Date.now()
      },
      ...request.messages
    ]
  }

  private generateConnectionId(): string {
    return `conn_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
  }

  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }
}

// conversation-manager.ts
export class ConversationManager {
  private client: LLMClient
  private messages: Message[] = []
  private systemPrompt: string
  private onMessageUpdate?: (messages: Message[]) => void

  constructor(
    client: LLMClient,
    systemPrompt: string = 'You are a helpful assistant.',
    onMessageUpdate?: (messages: Message[]) => void
  ) {
    this.client = client
    this.systemPrompt = systemPrompt
    this.onMessageUpdate = onMessageUpdate
  }

  /**
   * Add a user message and get streaming response
   */
  async sendMessage(
    content: string,
    options: {
      onToken?: (token: string) => void
      onComplete?: (response: string) => void
      onError?: (error: Error) => void
      abortController?: AbortController
    } = {}
  ): Promise<string> {
    // Add user message
    const userMessage: Message = {
      role: 'user',
      content,
      id: this.generateMessageId(),
      timestamp: Date.now()
    }
    
    this.messages.push(userMessage)
    
    // Add placeholder for assistant response
    const assistantMessage: Message = {
      role: 'assistant',
      content: '',
      id: this.generateMessageId(),
      timestamp: Date.now()
    }
    
    this.messages.push(assistantMessage)
    this.onMessageUpdate?.(this.messages)
    
    try {
      const response = await this.client.streamCompletion({
        messages: this.messages.slice(0, -1), // Don't include the empty assistant message
        systemPrompt: this.systemPrompt
      }, {
        onToken: (token) => {
          assistantMessage.content += token
          this.onMessageUpdate?.(this.messages)
          options.onToken?.(token)
        },
        onComplete: (fullResponse) => {
          assistantMessage.content = fullResponse
          this.onMessageUpdate?.(this.messages)
          options.onComplete?.(fullResponse)
        },
        onError: options.onError,
        abortController: options.abortController
      })
      
      return response
    } catch (error) {
      // Remove the failed assistant message
      this.messages.pop()
      this.onMessageUpdate?.(this.messages)
      throw error
    }
  }

  /**
   * Get all messages
   */
  getMessages(): Message[] {
    return [...this.messages]
  }

  /**
   * Get the last message
   */
  getLastMessage(): Message | null {
    return this.messages.length > 0 ? this.messages[this.messages.length - 1] : null
  }

  /**
   * Clear all messages
   */
  clearMessages(): void {
    this.messages = []
    this.onMessageUpdate?.(this.messages)
  }

  /**
   * Set system prompt
   */
  setSystemPrompt(prompt: string): void {
    this.systemPrompt = prompt
  }

  /**
   * Get current system prompt
   */
  getSystemPrompt(): string {
    return this.systemPrompt
  }

  private generateMessageId(): string {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
  }
}

// hypebot-manager.ts
export class HypebotManager {
  private client: LLMClient
  private systemPrompt: string
  private onUpdate?: (message: string) => void
  private currentMessage: string = ''

  constructor(
    client: LLMClient,
    onUpdate?: (message: string) => void
  ) {
    this.client = client
    this.systemPrompt = 'You are a 3rd party, omniscient observer to the plot. Your job is to provide a reaction to the following content. Be funny, provide suggestions, and be creative. You are allowed to criticize the plot, the user, and the writer. Do not answer questions, use plain formatting (no markdown, json, etc.), and keep your response within a single sentence. The content is:\n\n'
    this.onUpdate = onUpdate
  }

  /**
   * Generate streaming hypebot response
   */
  async generateResponse(
    content: string,
    options: {
      abortController?: AbortController
      onComplete?: (response: string) => void
      onError?: (error: Error) => void
    } = {}
  ): Promise<string> {
    this.currentMessage = ''
    this.onUpdate?.('Thinking...')

    try {
      const response = await this.client.streamCompletion({
        messages: [{
          role: 'user',
          content,
          timestamp: Date.now()
        }],
        systemPrompt: this.systemPrompt
      }, {
        onToken: (token) => {
          this.currentMessage += token
          this.onUpdate?.(this.currentMessage)
        },
        onComplete: (fullResponse) => {
          this.currentMessage = fullResponse
          this.onUpdate?.(this.currentMessage)
          options.onComplete?.(fullResponse)
        },
        onError: (error) => {
          this.onUpdate?.('Error getting response')
          options.onError?.(error)
        },
        abortController: options.abortController
      })

      return response
    } catch (error) {
      this.onUpdate?.('Error getting response')
      throw error
    }
  }

  /**
   * Get current message
   */
  getCurrentMessage(): string {
    return this.currentMessage
  }

  /**
   * Set custom system prompt
   */
  setSystemPrompt(prompt: string): void {
    this.systemPrompt = prompt
  }
}