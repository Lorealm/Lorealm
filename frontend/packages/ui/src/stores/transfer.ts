import { useApi, usePlatform } from '#lib/api/client.js';
import type { Snowflake } from '#lib/types/core.js';
import { extractError } from '#lib/types/error.js';
import type { TransferFile, TransferKind } from '#lib/types/transfer.js';
import { characterStore } from './character.svelte.js';
import { lorebookStore } from './lorebook.svelte.js';
import { showApiError } from './errorActions.js';
import { showToast } from './errorToast.svelte.js';
import { personaStore } from './persona.svelte.js';
import { showPopup } from './popup.svelte.js';
import { parseStyle, styles } from './styles.svelte.js';
import { templateStore } from './template.svelte.js';
import { toolStore } from './tools.svelte.js';
import { userWidgetStore } from './userWidgets.svelte.js';
import { t } from '#lib/stores/i18n.svelte.js';

const KINDS: string[] = [
	'template',
	'persona',
	'tool',
	'widget',
	'lorebook'
] satisfies TransferKind[];

function kindLabel(kind: Detected['kind']): string {
	return t.transfer.kinds[kind];
}

function fileStem(name: string, fallback: string): string {
	return name.replace(/[^A-Za-z0-9 _-]/g, '_').trim() || fallback;
}

/** Saves a template, persona, tool, widget or lorebook as a JSON file. */
export async function exportItem(kind: TransferKind, id: Snowflake, name: string) {
	try {
		const file = await useApi().transfer.export(kind, id);
		await usePlatform().files.save(JSON.stringify(file, null, '\t'), {
			label: t.transfer.fileLabel(kindLabel(kind)),
			extensions: ['json'],
			suggestedName: `${fileStem(name, kind)}.${kind}.json`
		});
	} catch (e) {
		showApiError(e, { title: t.transfer.exportFailed(kindLabel(kind)) });
	}
}

export async function exportSillyTavernLorebook(id: Snowflake, name: string) {
	try {
		const { file, warnings } = await useApi().lorebooks.exportSillyTavern(id);
		if (warnings.length > 0) {
			const download = t.transfer.download;
			const choice = await showPopup({
				title: t.transfer.sillyTavernTitle(name),
				message: [
					t.transfer.sillyTavernNote,
					t.transfer.wontCarry,
					warnings.map((w) => `- ${w.message}`).join('\n')
				].join('\n\n'),
				variant: 'warning',
				actions: [{ label: t.common.cancel }, { label: download, tone: 'primary' }]
			});
			if (choice !== download) return;
		}
		await usePlatform().files.save(JSON.stringify(file, null, '\t'), {
			label: t.transfer.sillyTavernLabel,
			extensions: ['json'],
			suggestedName: `${fileStem(name, t.transfer.lorebookFallback)}.json`
		});
	} catch (e) {
		showApiError(e, { title: t.transfer.lorebookExportFailed });
	}
}

type Detected =
	| { kind: 'character' }
	| { kind: 'style'; text: string }
	| { kind: 'SillyTavern lorebook'; data: unknown }
	| { kind: TransferKind; data: TransferFile };

async function detect(file: File): Promise<Detected | null> {
	if (file.type === 'image/png' || file.name.toLowerCase().endsWith('.png')) {
		return { kind: 'character' };
	}
	let text: string;
	let json: Record<string, unknown>;
	try {
		text = await file.text();
		json = JSON.parse(text);
	} catch {
		return null;
	}
	if (
		typeof json?.lorealm === 'number' &&
		typeof json.kind === 'string' &&
		KINDS.includes(json.kind)
	) {
		return { kind: json.kind as TransferKind, data: json as TransferFile };
	}
	if (typeof json?.spec === 'string' && json.spec.startsWith('chara_card')) {
		return { kind: 'character' };
	}
	if (typeof json?.entries === 'object' && json.entries !== null) {
		return { kind: 'SillyTavern lorebook', data: json };
	}
	if (
		typeof json?.name === 'string' &&
		(typeof json.css === 'string' || typeof json.tokens === 'object')
	) {
		return { kind: 'style', text };
	}
	return null;
}

/**
 * Works out what each file holds, asks to confirm the lot, then imports them.
 */
export async function importFiles(files: File[]) {
	const failed: string[] = [];
	const notes: string[] = [];
	const items: { file: File; detected: Detected }[] = [];
	for (const file of files) {
		const detected = await detect(file);
		if (detected) items.push({ file, detected });
		else failed.push(t.transfer.unrecognised(file.name));
	}

	if (items.length > 0) {
		const confirm = t.transfer.import;
		const choice = await showPopup({
			title:
				items.length === 1
					? t.transfer.importingAs(kindLabel(items[0].detected.kind))
					: t.transfer.importingFiles(items.length),
			message: items
				.map(({ file, detected }) => t.transfer.fileAs(file.name, kindLabel(detected.kind)))
				.join('\n'),
			actions: [{ label: t.common.cancel }, { label: confirm, tone: 'primary' }]
		});
		if (choice !== confirm) return;
	}

	let imported = 0;
	const touched = new Set<TransferKind>();

	// Cards go up together, the way the characters page sends them.
	const cards = items.filter((i) => i.detected.kind === 'character');
	if (cards.length > 0) {
		try {
			const results = await characterStore.import(
				cards.map((c) => c.file),
				true
			);
			results.forEach((r, i) => {
				if (r.status === 'ok') imported++;
				else failed.push(t.transfer.failedFile(cards[i].file.name, r.reason));
			});
		} catch (e) {
			const reason = extractError(e).message;
			for (const c of cards) failed.push(t.transfer.failedFile(c.file.name, reason));
		}
	}

	for (const { file, detected } of items) {
		if (detected.kind === 'character') continue;
		try {
			if (detected.kind === 'style') {
				await styles.add(parseStyle(detected.text));
			} else if (detected.kind === 'SillyTavern lorebook') {
				const name = file.name.replace(/\.json$/i, '');
				const book = await useApi().lorebooks.importSillyTavern(detected.data, name);
				touched.add('lorebook');
				if (book.skipped > 0) notes.push(t.transfer.entriesSkipped(book.skipped, book.name));
			} else {
				const { id, report } = await useApi().transfer.import(detected.data);
				touched.add(detected.kind);
				if (id === null) throw new Error(t.transfer.unreadable(report?.skipped.join(', ') ?? ''));
				if (report?.created === 0) notes.push(t.transfer.alreadyHad(file.name));
				for (const [from, to] of report?.renamed ?? []) notes.push(t.transfer.renamed(from, to));
			}
			imported++;
		} catch (e) {
			failed.push(t.transfer.failedFile(file.name, extractError(e).message));
		}
	}

	const summary = [t.transfer.summary(imported, files.length), ...notes].join(' ');
	if (failed.length === 0) {
		showToast({ title: t.transfer.imported, message: summary, variant: 'success' });
	} else {
		void showPopup({
			title: imported === 0 ? t.transfer.nothingImported : t.transfer.someFailed,
			message: `${summary}\n\n${failed.map((f) => `- ${f}`).join('\n')}`,
			variant: 'warning'
		});
	}

	try {
		if (touched.has('template')) await templateStore.loadList();
		if (touched.has('persona')) await personaStore.loadPersonas();
		if (touched.has('tool') || touched.has('widget')) await toolStore.loadTools();
		if (touched.has('widget')) await userWidgetStore.loadWidgets();
		if (touched.has('lorebook')) await lorebookStore.loadList();
	} catch (e) {
		showApiError(e, { title: t.transfer.refreshFailed });
	}
}
