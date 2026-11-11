import type { AttachmentReport } from './character.ts';
import type { Snowflake } from './core.ts';

export type TransferKind = 'template' | 'persona' | 'tool' | 'widget' | 'lorebook';

export interface TransferFile {
	lorealm: number;
	kind: TransferKind;
	[field: string]: unknown;
}

export interface TransferImportResponse {
	kind: TransferKind;
	id: Snowflake | null;
	report?: AttachmentReport;
}
