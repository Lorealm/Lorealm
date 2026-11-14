import { plural } from './plural.js';

/** The Characters page. */
export const characters = {
	chat: 'Chat',
	sort: 'Sort characters',
	sortOptions: {
		nameAsc: 'Name (A–Z)',
		nameDesc: 'Name (Z–A)',
		updated: 'Recently updated',
		created: 'Recently added'
	},
	reuseHint:
		'When a card brings a lorebook identical to one you have, attach yours instead of importing a copy.',
	reuse: 'Reuse matching lorebooks',
	uploading: 'Uploading…',
	import: 'Import',
	new: 'New',
	empty: 'No characters yet — create one to get started.',
	newName: 'New Character',
	createFailed: "Couldn't create character",
	attachments: {
		title: 'Lorebooks, tools and widgets',
		imported: (count: number) => `Imported ${count}.`,
		reused: (count: number) => `Reused ${count} you already had.`,
		rename: (from: string, to: string) => `${from} to ${to}`,
		renamed: (list: string) => `Renamed ${list}, since those names were taken.`,
		skipped: (list: string) => `Left out ${list}, which couldn't be read.`
	},
	lorebookImport: {
		title: 'Lorebook imported',
		named: (name: string) => `“${name}”`,
		lorebooks: (count: number) => `${count} lorebooks`,
		entries: (count: number) => plural(count, { one: `${count} entry`, other: `${count} entries` }),
		notConverted: (count: number) => ` ${count} could not be converted.`,
		message: (subject: string, entries: string, tail: string) =>
			`Imported ${subject} with ${entries}.${tail}`
	}
} as const;
