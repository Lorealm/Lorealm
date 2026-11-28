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
		details?: ErrorDetails;
	};
}

export type AppError = ParseError | ConfigError | HttpError | InternalError | ServerError;

/**
 * Resource kinds the server can point an error at. Mirrors `TargetType` in
 * `backend/server/src/errors/mod.rs`.
 */
export type ResourceKind =
	| 'character'
	| 'character_version'
	| 'greeting'
	| 'conversation'
	| 'message'
	| 'example_message'
	| 'context_template'
	| 'persona'
	| 'connection_profile'
	| 'secret'
	| 'user_tool'
	| 'user_widget'
	| 'lorebook'
	| 'realm'
	| 'realm_agent'
	| 'realm_version'
	| 'module'
	| 'scenario'
	| 'tokenizer'
	| 'lorebook_entry'
	| 'template_node'
	| 'role'
	| 'user';

/**
 * Machine-readable pointer to the resource an error concerns. Use this to route
 * instead of parsing `message`.
 */
export interface ErrorDetails {
	resource?: ResourceKind;
	id?: string;
}

/**
 * Every error code the server can emit. Mirrors the `DomainError::code`
 * implementations in `backend/server/src/errors/`; see `docs/errors.md`.
 */
export type ErrorCode =
	| 'internal'
	| 'auth.csrf_invalid'
	| 'auth.email_taken'
	| 'auth.invalid_credentials'
	| 'auth.invalid_invite_limit'
	| 'auth.invite_invalid'
	| 'auth.missing_token'
	| 'auth.registration_disabled'
	| 'auth.session_invalid'
	| 'auth.username_taken'
	| 'blob.unsupported_type'
	| 'character.invalid_card'
	| 'character.malformed_upload'
	| 'character.no_files'
	| 'character.not_found'
	| 'character.unsupported_format'
	| 'character.unsupported_media_type'
	| 'chat.bad_image'
	| 'chat.not_found'
	| 'chat.provider_failed'
	| 'chat.realm_conversation'
	| 'command.bad_argument'
	| 'command.cancelled'
	| 'command.failed'
	| 'command.not_a_command'
	| 'command.not_found'
	| 'command.realm_conversation'
	| 'config.unknown_key'
	| 'connection.invalid_profile'
	| 'connection.not_found'
	| 'connection.secret_corrupted'
	| 'context.evaluation_failed'
	| 'context.no_active_character'
	| 'context.not_found'
	| 'context.plugin_failed'
	| 'context.resolver_not_found'
	| 'context.unknown_source_node'
	| 'conversation.message_not_found'
	| 'conversation.not_found'
	| 'lorebook.evaluation_failed'
	| 'lorebook.invalid_yaml'
	| 'lorebook.not_found'
	| 'module.invalid'
	| 'module.name_taken'
	| 'module.not_found'
	| 'permission.built_in_role'
	| 'permission.denied'
	| 'permission.invalid_role_name'
	| 'permission.invalid_role_position'
	| 'permission.last_administrator'
	| 'permission.own_account'
	| 'permission.role_name_taken'
	| 'permission.role_not_found'
	| 'permission.role_outranked'
	| 'permission.user_not_found'
	| 'permission.user_outranked'
	| 'persona.not_found'
	| 'persona.unsupported_media_type'
	| 'realm.action_failed'
	| 'realm.import_in_use'
	| 'realm.invalid_agent'
	| 'realm.invalid_scenario'
	| 'realm.invalid_turn'
	| 'realm.malformed_agent'
	| 'realm.no_agents'
	| 'realm.no_runnable_agent'
	| 'realm.not_found'
	| 'realm.not_imported'
	| 'realm.not_realm_conversation'
	| 'realm.not_your_turn'
	| 'realm.tool_not_bound'
	| 'state.no_active_connection_profile'
	| 'state.no_active_persona'
	| 'state.no_active_template'
	| 'state.no_active_tokenizer'
	| 'state.not_found'
	| 'template.circular_reference'
	| 'template.duplicate_source_node'
	| 'template.index_out_of_bounds'
	| 'template.invalid_parent'
	| 'template.invalid_path'
	| 'template.node_not_found'
	| 'tokenizer.empty_file'
	| 'tokenizer.fetch_failed'
	| 'tokenizer.forbidden'
	| 'tokenizer.invalid_data'
	| 'tokenizer.invalid_source'
	| 'tokenizer.not_found'
	| 'tokenizer.tokenize_failed'
	| 'tool.invalid_body'
	| 'tool.invalid_name'
	| 'tool.not_found'
	| 'widget.invalid'
	| 'widget.not_found';

/** Namespace part of an {@link ErrorCode} */
export type ErrorDomain = ErrorCode extends `${infer D}.${string}` ? D : never;

export function errorDomain(code: string | undefined): ErrorDomain | undefined {
	const domain = code?.split('.')[0];
	return domain && domain !== code ? (domain as ErrorDomain) : undefined;
}

/** A thrown value normalised into the server's error contract. */
export interface ExtractedError {
	message: string;
	code?: ErrorCode;
	details?: ErrorDetails;
	/**
	 * HTTP status. `404` means `details.id` names something that is gone.
	 *
	 * Named apart from `status` so this can be spread into state objects that
	 * carry a status of their own.
	 */
	httpStatus?: number;
}

/**
 * Normalises anything thrown by the API client.
 */
export function extractError(e: unknown): ExtractedError {
	if (e instanceof Error) {
		const err = e as Error & { code?: unknown; details?: unknown; status?: unknown };
		return {
			message: e.message,
			code: typeof err.code === 'string' ? (err.code as ErrorCode) : undefined,
			details: err.details as ErrorDetails | undefined,
			httpStatus: typeof err.status === 'number' ? err.status : undefined
		};
	}
	if (e && typeof e === 'object') {
		const err = e as Record<string, unknown>;
		return {
			message: typeof err.message === 'string' ? err.message : JSON.stringify(err),
			code: typeof err.error_code === 'string' ? (err.error_code as ErrorCode) : undefined,
			details: err.details as ErrorDetails | undefined,
			httpStatus: typeof err.status === 'number' ? err.status : undefined
		};
	}
	return { message: String(e) };
}
