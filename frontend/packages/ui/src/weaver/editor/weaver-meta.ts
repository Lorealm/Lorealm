import type { Messages } from '@lorealm/i18n';
import { t } from '#lib/stores/i18n.svelte.js';

export interface DocEntry {
	signature: string;
	doc: string;
}

type Keyword = keyof Messages['weaver']['keywords'];

/** The docs follow the active locale; signatures are notation and stay as written. */
const entry = (name: Keyword, signature: string): DocEntry => ({
	signature,
	get doc() {
		return t.weaver.keywords[name];
	}
});

export const keywords: Record<string, DocEntry> = {
	if: entry('if', '{# if <expr> #} ... {# endif #}'),
	elif: entry('elif', '{# elif <expr> #}'),
	else: entry('else', '{# else #}'),
	endif: entry('endif', '{# endif #}'),
	foreach: entry('foreach', '{# foreach <name> in <array> #} ... {# endforeach #}'),
	endforeach: entry('endforeach', '{# endforeach #}'),
	in: entry('in', 'foreach <name> in <array>'),
	break: entry('break', '{# break #}'),
	continue: entry('continue', '{# continue #}'),
	return: entry('return', '{# return <expr> #}'),
	stop: entry('stop', '{# stop #}')
};
