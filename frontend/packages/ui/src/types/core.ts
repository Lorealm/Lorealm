/** String-formatted Snowflake value. Server handles Snowflake
 * (de)serialization from strings, so this value may be used
 * directly.
 *
 * JavaScript is very smart and capable will truncate full
 * integer-formatted snowflakes as it can only represent ~53 bits
 * worth of integers.
 */
export type Snowflake = string;

export type PromptRole = 'system' | 'user' | 'assistant';

/**
 * Arbitrary JSON value
 */
export type JsonValue =
	null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
