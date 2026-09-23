/** One place in the client a plugin fills. */
export interface FrontendSlot {
	/** Which slot, such as `inspector`. */
	slot: string;
	/** The HTML file the slot shows. */
	entry: string;
	title: string;
	/** Runs in the client itself, from an ES module exporting `mount`. */
	unsafe: boolean;
	/** For `message` slots: on every message, or only the latest. */
	mode: 'latest' | 'every';
}

export interface InstalledPlugin {
	handle: string;
	name: string;
	description: string;
	tags: string[];
	author: string;
	version: string;
	permissions: string[];
	enabled: boolean;
	slots: FrontendSlot[];
}
