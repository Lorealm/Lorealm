import type { User } from "./user.ts";

export interface LoginRequest {
	username: string;
	password: string;
}

export interface Session {
	token: string;
	user: User;
}
