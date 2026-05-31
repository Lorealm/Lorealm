// Pure conversion between a `Slot` value and the flat representation a picker
// binds to. Two wrinkles the UI needs handled here, not in markup:
//   - `at_depth` is an object variant; the picker shows a number field for N.
//   - an absent slot (`undefined`) means "inherit the lorebook default" and
//     must round-trip back to `undefined`, so the serializer never writes the
//     key unless the author explicitly chose a slot.

import { SLOT_KINDS, isAtDepth, type Slot } from '../types/lorebook.js';

/** A picker choice: the slot kinds plus an explicit "inherit default". */
export type SlotChoice = 'inherit' | (typeof SLOT_KINDS)[number];

export const SLOT_CHOICES: SlotChoice[] = ['inherit', ...SLOT_KINDS];

export interface SlotFormValue {
	choice: SlotChoice;
	/** Only meaningful when `choice === 'at_depth'`. */
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

/** Human label for a resolved slot, e.g. for showing the inherited default. */
export function slotLabel(slot: Slot | undefined): string {
	if (slot === undefined) return 'inherit';
	if (isAtDepth(slot)) return `at depth ${slot.at_depth}`;
	return slot;
}
