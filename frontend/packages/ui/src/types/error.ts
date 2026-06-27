export interface ParseError {
	error_type: 'Parse';
	data: string;
}

export interface ConfigError {
	error_type: 'Config';
	data: string;
}

export interface HttpError {
	error_type: 'Http';
	data: string;
}

export interface InternalError {
	error_type: 'Internal';
	data: string;
}

export interface ServerError {
	error_type: 'Server';
	data: {
		error_code: string;
		message: string;
	};
}

export type AppError = ParseError | ConfigError | HttpError | InternalError | ServerError;
