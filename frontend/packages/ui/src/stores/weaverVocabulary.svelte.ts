import { useApi, type ApiClient } from '$lib/api/client.js';
import type {
	CommandSignature,
	ParamDef,
	ProcessorSignature,
	PropertyDef
} from '$lib/types/weaver.js';
import type { ParamDoc } from '$lib/weaver/editor/weaver-render.js';

export function processorKey(proc: ProcessorSignature): string {
	return `${proc.namespace}.${proc.name}`;
}

function formatParam(p: ParamDef | PropertyDef): string {
	const name = 'name' in p ? p.name : p.key;
	const type = p.expected_type ?? 'any';
	return `${name}${p.required ? '' : '?'}: ${type}`;
}

export function formatCommandSignature(cmd: CommandSignature): string {
	return `${cmd.name}(${cmd.params.map(formatParam).join(', ')}) -> ${cmd.returns}`;
}

export function formatProcessorSignature(proc: ProcessorSignature): string {
	return `${processorKey(proc)}(${proc.properties.map(formatParam).join(', ')}) -> ${proc.returns}`;
}

export function paramDocs(sig: CommandSignature | ProcessorSignature): ParamDoc[] {
	const params: Array<ParamDef | PropertyDef> = 'params' in sig ? sig.params : sig.properties;
	return params
		.filter((p) => p.description)
		.map((p) => ({ name: 'name' in p ? p.name : p.key, description: p.description }));
}

class WeaverVocabularyStore {
	private _api = $state<ApiClient>();

	commands = $state<CommandSignature[]>([]);
	processors = $state<ProcessorSignature[]>([]);
	loaded = $state(false);

	/** In-flight fetch, so concurrent editors don't each hit the endpoint. */
	private pending: Promise<void> | null = null;

	get #api() {
		if (!this._api) this._api = useApi();
		return this._api;
	}

	/** Look up a command by its bare name (`set_var`). */
	command(name: string): CommandSignature | undefined {
		return this.commands.find((c) => c.name === name);
	}

	/** Look up a processor by its `namespace.name` key (`text.upper`). */
	processor(key: string): ProcessorSignature | undefined {
		return this.processors.find((p) => processorKey(p) === key);
	}

	/** Fetch the vocabulary once. Repeat calls reuse the first result. */
	async load(): Promise<void> {
		if (this.loaded) return;
		if (this.pending) return this.pending;

		this.pending = this.#api.templates
			.weaverVocabulary()
			.then((vocab) => {
				this.commands = vocab.commands;
				this.processors = vocab.processors;
				this.loaded = true;
			})
			.finally(() => {
				this.pending = null;
			});

		return this.pending;
	}
}

export const weaverVocabularyStore = new WeaverVocabularyStore();
