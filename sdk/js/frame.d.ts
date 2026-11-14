/**
 * Declares `window.lorealm` for scripts in a frame slot.
 *
 * ```ts
 * /// <reference types="@lorealm/plugin/frame" />
 * ```
 *
 * `lorealm.context` is typed as {@link SlotContext}, which every slot
 * satisfies. For one slot's exact context, cast it:
 *
 * ```ts
 * import type { LorealmApi, MessageContext } from '@lorealm/plugin';
 * const api = lorealm as LorealmApi<MessageContext>;
 * ```
 *
 * @packageDocumentation
 */

import type { LorealmApi, SlotContext } from './index.js';

declare global {
	/**
	 * The Official Lorealm Client's API.
	 *
	 * The frame is sandboxed: it has no network access, and loads nothing from
	 * outside its HTML file. It also has no cookies or `localStorage`; keep state
	 * in the plugin's backend.
	 *
	 * The frame gets the client's CSS custom properties, such as
	 * `--color-text-primary`, along with the client's fonts. These
	 * are automatically updated whenever the theme changes. You don't
	 * need to worry about it.
	 *
	 * For a complete overview over wht CSS variables are available, see
	 * the [source code](https://github.com/Lorealm/Lorealm/blob/main/frontend/packages/ui/src/assets/css/variables.css).
	 *
	 * Note that only the variables.css file is currently injected. Global element
	 * styles are not (yet) injected.
	 */
	var lorealm: LorealmApi<SlotContext>;
}

export {};
