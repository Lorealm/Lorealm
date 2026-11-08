export type PopupVariant = 'info' | 'warning' | 'danger';
export type PopupActionTone = 'neutral' | 'primary' | 'danger';

export interface PopupAction {
	label: string;
	/** Defaults to `neutral`. */
	tone?: PopupActionTone;
	/** Runs when the action is clicked, before the popup closes. */
	run?: () => void | Promise<void>;
}

export interface PopupInput {
	title: string;
	/** The message to show. Supports markdown. */
	message: string;
	/** Defaults to `info`. */
	variant?: PopupVariant;
	/** Defaults to a single "OK". */
	actions?: PopupAction[];
	/** Whether Escape, the close button and clicking outside dismiss it. Defaults to true. */
	dismissible?: boolean;
}

export interface Popup extends Required<PopupInput> {
	id: number;
	/** Settles the promise `showPopup` returned. */
	settle: (label: string | null) => void;
}

let nextId = 1;

/** The queue; the first entry is on screen. */
export const popups = $state<Popup[]>([]);

/**
 * Opens a popup. Resolves with the label of the action clicked, or `null` if it
 * was dismissed.
 */
export function showPopup(input: PopupInput): Promise<string | null> {
	return new Promise((resolve) => {
		popups.push({
			id: nextId++,
			title: input.title,
			message: input.message,
			variant: input.variant ?? 'info',
			actions: input.actions?.length ? input.actions : [{ label: 'OK', tone: 'primary' }],
			dismissible: input.dismissible ?? true,
			settle: resolve
		});
	});
}

/** Closes the popup with that id, settling it with `label`. */
export function closePopup(id: number, label: string | null) {
	const index = popups.findIndex((p) => p.id === id);
	if (index === -1) return;
	const [popup] = popups.splice(index, 1);
	popup.settle(label);
}
