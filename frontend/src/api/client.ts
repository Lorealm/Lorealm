import type { ApiErrorResponse } from '@/types/auth'

class ApiClient {
  private baseUrl: string
  private accessToken: string | null = null
  private csrfToken: string | null = null
  private onCsrfTokenUpdate: ((token: string) => void) | null = null

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl
    this.accessToken = localStorage.getItem('access_token')
    this.csrfToken = localStorage.getItem('csrf_token')
  }

  setAccessToken(token: string): void {
    this.accessToken = token
    localStorage.setItem('access_token', token)
  }

  setCsrfToken(token: string): void {
    this.csrfToken = token
    localStorage.setItem('csrf_token', token)
    this.onCsrfTokenUpdate?.(token)
  }

  getCsrfToken(): string | null {
    return this.csrfToken
  }

  onCsrfUpdate(callback: (token: string) => void): void {
    this.onCsrfTokenUpdate = callback
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<{ data: T; newCsrfToken?: string }> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(typeof options.headers === 'object' && options.headers !== null
        ? (options.headers as Record<string, string>)
        : {}),
    }

    if (this.accessToken) {
      headers['Authorization'] = `Bearer ${this.accessToken}`
    }

    if (this.csrfToken && this.isStateChangingRequest(options.method)) {
      headers['X-CSRF-Token'] = this.csrfToken
    }

    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      headers,
      credentials: 'include', // Important for cookie-based session
    })

    if (!response.ok) {
      const errorData: ApiErrorResponse = await response.json()
      const error = new Error(errorData.error) as Error & { errorType?: string }
      error.errorType = errorData.error_type
      throw error
    }

    const responseData = await response.json()

    // Extract new CSRF token if provided
    let newCsrfToken: string | undefined
    if (responseData.new_csrf_token) {
      newCsrfToken = responseData.new_csrf_token
      this.setCsrfToken(newCsrfToken!)
    }

    return {
      data: responseData.data ?? responseData,
      newCsrfToken,
    }
  }

  private isStateChangingRequest(method?: string): boolean {
    const stateChangingMethods = ['POST', 'PUT', 'PATCH', 'DELETE']
    return stateChangingMethods.includes(method?.toUpperCase() ?? 'GET')
  }

  async get<T>(endpoint: string): Promise<T> {
    const { data } = await this.request<T>(endpoint, { method: 'GET' })
    return data
  }

  async post<T>(endpoint: string, body: unknown): Promise<T> {
    const { data } = await this.request<T>(endpoint, {
      method: 'POST',
      body: JSON.stringify(body),
    })
    return data
  }

  async put<T>(endpoint: string, body: unknown): Promise<T> {
    const { data } = await this.request<T>(endpoint, {
      method: 'PUT',
      body: JSON.stringify(body),
    })
    return data
  }

  async patch<T>(endpoint: string, body: unknown): Promise<T> {
    const { data } = await this.request<T>(endpoint, {
      method: 'PATCH',
      body: JSON.stringify(body),
    })
    return data
  }

  async delete<T>(endpoint: string): Promise<T> {
    const { data } = await this.request<T>(endpoint, { method: 'DELETE' })
    return data
  }
}

export const apiClient = new ApiClient(process.env.VITE_API_URL || '')
