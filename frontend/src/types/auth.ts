export interface User {
  id: string
  email: string | undefined
  username: string
}

export interface LoginResponse {
  csrf_token: string
  user_id: string
}

export interface ApiResponse<T> {
  data: T
  new_csrf_token?: string
}

export interface ApiErrorResponse {
  error: string
  error_type: string
}

export interface AuthError {
  code: string
  message: string
  errorType?: string
}
