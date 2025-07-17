export interface Message {
  role: 'system' | 'user' | 'assistant'
  content: string
  id?: string
  timestamp?: number
}

export interface StreamingOptions {
  onToken?: (token: string) => void
  onComplete?: (fullResponse: string) => void
  onError?: (error: Error) => void
  abortController?: AbortController
}

export interface LLMClientConfig {
  baseUrl?: string
  defaultSystemPrompt?: string
  timeout?: number
  retryAttempts?: number
  retryDelay?: number
}

export interface CompletionRequest {
  messages: Message[]
  systemPrompt?: string
  temperature?: number
  maxTokens?: number
  stream?: boolean
}