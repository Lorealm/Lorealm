import { SLOT_KINDS, isAtDepth, type Slot } from '../types/lorebook.js';
import { t } from '#lib/stores/i18n.svelte.js';

export type SlotChoice = 'inherit' | (typeof SLOT_KINDS)[number];

export const SLOT_CHOICES: SlotChoice[] = ['inherit', ...SLOT_KINDS];

export interface SlotFormValue {
	choice: SlotChoice;
	depth: number;
}

export function slotToForm(slot: Slot | undefined): SlotFormValue {
	if (slot === undefined) return { choice: 'inherit', depth: 0 };
	if (isAtDepth(slot)) return { choice: 'at_depth', depth: slot.at_depth };
	return { choice: slot, depth: 0 };
}

export function formToSlot(form: SlotFormValue): Slot | undefined {
	if (form.choice === 'inherit') return undefined;
	if (form.choice === 'at_depth') return { at_depth: form.depth };
	return form.choice;
}

/** Human label for a resolved slot */
export function slotLabel(slot: Slot | undefined): string {
	if (slot === undefined) return t.studio.slot.inherit;
	if (isAtDepth(slot)) return t.studio.slot.atDepth({ depth: slot.at_depth });
	return slot;
}
