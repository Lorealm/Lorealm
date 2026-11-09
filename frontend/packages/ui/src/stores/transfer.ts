import { useApi, usePlatform } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import { extractError } from '$lib/types/error.js';
import type { TransferFile, TransferKind } from '$lib/types/transfer.js';
import { characterStore } from './character.svelte.js';
import { showApiError } from './errorActions.js';
import { showToast } from './errorToast.svelte.js';
import { personaStore } from './persona.svelte.js';
import { showPopup } from './popup.svelte.js';
import { parseStyle, styles } from './styles.svelte.js';
import { templateStore } from './template.svelte.js';
import { toolStore } from './tools.svelte.js';
import { userWidgetStore } from './userWidgets.svelte.js';

const KINDS: string[] = ['template', 'persona', 'tool', 'widget'] satisfies TransferKind[];

/** Saves a template, persona, tool or widget as a JSON file. */
export async function exportItem(kind: TransferKind, id: Snowflake, name: string) {
	try {
		const file = await useApi().transfer.export(kind, id);
		const stem = name.replace(/[^A-Za-z0-9 _-]/g, '_').trim() || kind;
		await usePlatform().files.save(JSON.stringify(file, null, '\t'), {
			label: `Lorealm ${kind}`,
			extensions: ['json'],
			suggestedName: `${stem}.${kind}.json`
		});
	} catch (e) {
		showApiError(e, { title: `Could not export ${kind}` });
	}
}

type Detected =
	| { kind: 'character' }
	| { kind: 'style'; text: string }
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
	// Style files carry no marker, only their fields.
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
 * Files that fail are left out and reported once everything has run.
 */
export async function importFiles(files: File[]) {
	const failed: string[] = [];
	const notes: string[] = [];
	const items: { file: File; detected: Detected }[] = [];
	for (const file of files) {
		const detected = await detect(file);
		if (detected) items.push({ file, detected });
		else failed.push(`**${file.name}**: not a character card, style or Lorealm export.`);
	}

	if (items.length > 0) {
		const choice = await showPopup({
			title:
				items.length === 1
					? `Importing as ${items[0].detected.kind}`
					: `Importing ${items.length} files`,
			message: items
				.map(({ file, detected }) => `- **${file.name}** as ${detected.kind}`)
				.join('\n'),
			actions: [{ label: 'Cancel' }, { label: 'Import', tone: 'primary' }]
		});
		if (choice !== 'Import') return;
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
				else failed.push(`**${cards[i].file.name}**: ${r.reason}`);
			});
		} catch (e) {
			const reason = extractError(e).message;
			for (const c of cards) failed.push(`**${c.file.name}**: ${reason}`);
		}
	}

	for (const { file, detected } of items) {
		if (detected.kind === 'character') continue;
		try {
			if (detected.kind === 'style') {
				await styles.add(parseStyle(detected.text));
			} else {
				const { id, report } = await useApi().transfer.import(detected.data);
				touched.add(detected.kind);
				if (id === null) throw new Error(`${report?.skipped.join(', ')} couldn't be read.`);
				if (report?.created === 0) notes.push(`You already had ${file.name}.`);
				for (const [from, to] of report?.renamed ?? [])
					notes.push(`${from} was renamed to ${to}, since that name was taken.`);
			}
			imported++;
		} catch (e) {
			failed.push(`**${file.name}**: ${extractError(e).message}`);
		}
	}

	const summary = [`Imported ${imported} of ${files.length}.`, ...notes].join(' ');
	if (failed.length === 0) {
		showToast({ title: 'Imported', message: summary, variant: 'success' });
	} else {
		void showPopup({
			title: imported === 0 ? 'Nothing was imported' : 'Some files were not imported',
			message: `${summary}\n\n${failed.map((f) => `- ${f}`).join('\n')}`,
			variant: 'warning'
		});
	}

	try {
		if (touched.has('template')) await templateStore.loadList();
		if (touched.has('persona')) await personaStore.loadPersonas();
		if (touched.has('tool') || touched.has('widget')) await toolStore.loadTools();
		if (touched.has('widget')) await userWidgetStore.loadWidgets();
	} catch (e) {
		showApiError(e, { title: 'Could not refresh after importing' });
	}
}
