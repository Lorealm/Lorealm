import {
	Decoration,
	EditorView,
	ViewPlugin,
	type DecorationSet,
	type PluginValue,
	type ViewUpdate
} from '@codemirror/view';
import { RangeSetBuilder, StateEffect, StateField, type Extension } from '@codemirror/state';
import {
	referenceSource,
	scanIdRefs,
	type EntryRef,
	type ReferenceSource
} from './weaver-references.js';

const linkMark = Decoration.mark({ class: 'cm-weaver-ref' });

function isMod(event: KeyboardEvent | MouseEvent): boolean {
	return event.metaKey || event.ctrlKey;
}

function target(view: EditorView, id: string): EntryRef | null {
	const source = view.state.facet(referenceSource);
	if (!source?.navigate) return null;
	const hit = source.entries().find((e) => e.id === id);
	return hit && hit.navigable !== false ? hit : null;
}

const setModHeld = StateEffect.define<boolean>();

const modHeld = StateField.define<boolean>({
	create: () => false,
	update(value, tr) {
		for (const effect of tr.effects) if (effect.is(setModHeld)) value = effect.value;
		return value;
	}
});

function linkDecorations(view: EditorView): DecorationSet {
	const builder = new RangeSetBuilder<Decoration>();
	const source = view.state.facet(referenceSource);
	if (!source?.navigate) return builder.finish();

	const navigable = new Set(
		source
			.entries()
			.filter((e) => e.navigable !== false)
			.map((e) => e.id)
	);
	if (navigable.size === 0) return builder.finish();

	for (const { from, to } of view.visibleRanges) {
		for (let pos = from; pos <= to;) {
			const line = view.state.doc.lineAt(pos);
			for (const ref of scanIdRefs(line.text)) {
				if (navigable.has(ref.id)) {
					builder.add(line.from + ref.from, line.from + ref.to, linkMark);
				}
			}
			pos = line.to + 1;
		}
	}
	return builder.finish();
}

const refLinkPlugin = ViewPlugin.fromClass(
	class implements PluginValue {
		decorations: DecorationSet = Decoration.none;

		constructor(private view: EditorView) {
			window.addEventListener('keydown', this.onKey);
			window.addEventListener('keyup', this.onKey);
			window.addEventListener('blur', this.onBlur);
		}

		destroy() {
			window.removeEventListener('keydown', this.onKey);
			window.removeEventListener('keyup', this.onKey);
			window.removeEventListener('blur', this.onBlur);
		}

		update(update: ViewUpdate) {
			const held = update.state.field(modHeld);
			const toggled = held !== update.startState.field(modHeld);
			if (toggled) update.view.dom.classList.toggle('cm-weaver-mod', held);
			if (!held) this.decorations = Decoration.none;
			else if (toggled || update.docChanged || update.viewportChanged) {
				this.decorations = linkDecorations(update.view);
			}
		}

		private onKey = (event: KeyboardEvent) => this.setMod(isMod(event));
		private onBlur = () => this.setMod(false);

		private setMod(held: boolean) {
			if (held !== this.view.state.field(modHeld)) {
				this.view.dispatch({ effects: setModHeld.of(held) });
			}
		}
	},
	{ decorations: (plugin) => plugin.decorations }
);

const refClickHandler = EditorView.domEventHandlers({
	mousedown(event, view) {
		if (event.button !== 0 || !isMod(event)) return false;

		const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
		if (pos == null) return false;

		const line = view.state.doc.lineAt(pos);
		const col = pos - line.from;
		const ref = scanIdRefs(line.text).find((r) => col >= r.from && col <= r.to);
		if (!ref) return false;

		const entry = target(view, ref.id);
		if (!entry) return false;

		// Claim the event so the click doesn't also move the cursor.
		event.preventDefault();
		view.state.facet(referenceSource)!.navigate!(entry);
		return true;
	}
});

export function weaverReferences(source: ReferenceSource): Extension {
	return [referenceSource.of(source), modHeld, refLinkPlugin, refClickHandler];
}
