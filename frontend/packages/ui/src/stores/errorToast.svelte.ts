import { SvelteMap } from "svelte/reactivity";

export type ToastAction =
	| {
			kind: 'callback';
			label: string;
			run: () => void;
	  }
	| {
			kind: 'scroll';
			label: string;
			selector: string;
			behavior?: ScrollBehavior;
	  };

export type ToastVariant = 'error' | 'info' | 'warning' | 'success';

export type Toast = {
	id: number;
	title: string;
	message: string;
	variant: ToastVariant;
	durationMs: number; // 0 = sticky
	action?: ToastAction;
};

type ShowToastInput = {
	title?: string;
	message: string;
	variant?: ToastVariant;
	durationMs?: number;
	action?: ToastAction;
};

let nextId = 1;
const timers = new SvelteMap<number, ReturnType<typeof setTimeout>>();

export const toasts = $state<Toast[]>([]);

function scheduleDismiss(id: number, durationMs: number) {
	if (durationMs <= 0) return;

	const timer = setTimeout(() => {
		dismissToast(id);
	}, durationMs);

	timers.set(id, timer);
}

export function showToast(input: ShowToastInput) {
	const toast: Toast = {
		id: nextId++,
		title: input.title ?? 'Notification',
		message: input.message,
		variant: input.variant ?? 'info',
		durationMs: input.durationMs ?? 6000,
		action: input.action
	};

	toasts.push(toast);
	scheduleDismiss(toast.id, toast.durationMs);

	return toast.id;
}

export function showError(
	error: unknown,
	options: Omit<ShowToastInput, 'message' | 'variant'> = {}
) {
	const message =
		typeof error === 'string'
			? error
			: error instanceof Error
				? error.message
				: 'Something went wrong';

	return showToast({
		...options,
		message,
		variant: 'error'
	});
}

export function dismissToast(id: number) {
	const timer = timers.get(id);
	if (timer) {
		clearTimeout(timer);
		timers.delete(id);
	}

	const index = toasts.findIndex((t) => t.id === id);
	if (index !== -1) toasts.splice(index, 1);
}

export function clearToasts() {
	for (const timer of timers.values()) clearTimeout(timer);
	timers.clear();
	toasts.splice(0, toasts.length);
}
