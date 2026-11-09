import type { AttachmentReport } from './character.ts';
import type { Snowflake } from './core.ts';

export type TransferKind = 'template' | 'persona' | 'tool' | 'widget';

/** An exported template, persona, tool or widget, as written to disk. */
export interface TransferFile {
	lorealm: number;
	kind: TransferKind;
	[field: string]: unknown;
}

export interface TransferImportResponse {
	kind: TransferKind;
	/** Null when a tool or widget was left out; the report says why. */
	id: Snowflake | null;
	/** Present for tools and widgets. */
	report?: AttachmentReport;
}
