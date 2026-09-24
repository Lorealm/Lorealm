import type { InputSpec } from './blueprint.js';

export interface PluginSettings {
	inputs: InputSpec[];
	values: Record<string, unknown>;
}

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

/** What installing a plugin from source produced. */
export interface InstallReport {
	handle: string;
	version: string;
	/** What the build commands printed. */
	log: string;
}

export interface InstalledPlugin {
	handle: string;
	name: string;
	description: string;
	tags: string[];
	author: string;
	version: string;
	permissions: string[];
	/** If so, the fields above describe the version the user is on. */
	enabled: boolean;
	/** The newest version that can be enabled; newer than `version` when an update is available. */
	latest: string | null;
	slots: FrontendSlot[];
	/** The API key types it declares. */
	api_keys: string[];
	/** Whether it has settings for the user to fill in. */
	has_settings: boolean;
	/** Where it may send requests. */
	hosts: string[];
}
