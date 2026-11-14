import { plural } from './plural.js';

/** Exporting and importing files. */
export const transfer = {
	kinds: {
		character: 'character',
		style: 'style',
		'SillyTavern lorebook': 'SillyTavern lorebook',
		template: 'template',
		persona: 'persona',
		tool: 'tool',
		widget: 'widget',
		lorebook: 'lorebook'
	},
	fileLabel: (kind: string) => `Lorealm ${kind}`,
	exportFailed: (kind: string) => `Could not export ${kind}`,
	sillyTavernTitle: (name: string) => `Export ${name} for SillyTavern`,
	sillyTavernNote:
		'Lorealm-only features come back when the file is imported into Lorealm again, unless a site removed them.',
	wontCarry: "**What won't carry over:**",
	download: 'Download',
	sillyTavernLabel: 'SillyTavern lorebook',
	lorebookFallback: 'lorebook',
	lorebookExportFailed: 'Could not export lorebook',
	unrecognised: (file: string) =>
		`**${file}**: not a character card, lorebook, style or Lorealm export.`,
	importingAs: (kind: string) => `Importing as ${kind}`,
	importingFiles: (count: number) => `Importing ${count} files`,
	fileAs: (file: string, kind: string) => `- **${file}** as ${kind}`,
	import: 'Import',
	failedFile: (file: string, reason: string) => `**${file}**: ${reason}`,
	entriesSkipped: (count: number, book: string) =>
		plural(count, {
			one: `${count} entry in ${book} couldn't be read and was left out.`,
			other: `${count} entries in ${book} couldn't be read and were left out.`
		}),
	unreadable: (list: string) => `${list} couldn't be read.`,
	alreadyHad: (file: string) => `You already had ${file}.`,
	renamed: (from: string, to: string) => `${from} was renamed to ${to}, since that name was taken.`,
	summary: (imported: number, total: number) => `Imported ${imported} of ${total}.`,
	imported: 'Imported',
	nothingImported: 'Nothing was imported',
	someFailed: 'Some files were not imported',
	refreshFailed: 'Could not refresh after importing'
} as const;
