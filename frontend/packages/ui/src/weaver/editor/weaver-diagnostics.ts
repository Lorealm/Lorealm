import { linter, type Diagnostic } from '@codemirror/lint';
import type { Extension } from '@codemirror/state';
import { useApi } from '#lib/api/client.js';
import type { WeaverDialect, WeaverDiagnostic, WeaverProfile } from '#lib/types/weaver.js';

const DELAY = 400;

/**
 * Position a server diagnostic against the document as it stands now.
 */
function toDiagnostic(server: WeaverDiagnostic, length: number): Diagnostic {
	const from = Math.min(server.from, length);
	const to = Math.min(Math.max(server.to, from), length);
	return {
		from,
		to,
		severity: server.severity,
		message: server.hint ? `${server.message}\n\nHint: ${server.hint}` : server.message
	};
}

/**
 * Underline what the backend cannot compile.
 */
export function weaverDiagnostics(
	dialect: WeaverDialect = 'template',
	profile?: WeaverProfile
): Extension {
	return linter(
		async (view) => {
			const source = view.state.doc.toString();
			if (source.trim() === '') return [];

			let found: WeaverDiagnostic[];
			try {
				found = (await useApi().templates.weaverCheck(source, dialect, profile)).diagnostics;
			} catch {
				return [];
			}

			const length = view.state.doc.length;
			return found.map((server) => toDiagnostic(server, length));
		},
		{ delay: DELAY }
	);
}
