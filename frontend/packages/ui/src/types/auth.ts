import type { User } from './user.js';

export interface LoginRequest {
	username: string;
	password: string;
}

export interface Session {
	token: string;
	user: User;
}
