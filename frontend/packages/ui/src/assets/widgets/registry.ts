import type { Component } from 'svelte';
import SmsOutgoing from './sms/SmsOutgoing.svelte';
import SmsIncoming from './sms/SmsIncoming.svelte';
import Note from './note/Note.svelte';
import SignMedieval from './sign/SignMedieval.svelte';
import NoteParchment from './note/NoteParchment.svelte';
import SignSteampunk from './sign/SignSteampunk.svelte';
import SignNeon from './sign/SignNeon.svelte';
import NoteArcane from './note/NoteArcane.svelte';

export type WidgetComponentProps = { props: Record<string, unknown> };

export const widgetRegistry: Record<string, Component<WidgetComponentProps>> = {
	'sms-outgoing': SmsOutgoing,
	'sms-incoming': SmsIncoming,
	note: Note,
	'sign-medieval': SignMedieval,
	'note-parchment': NoteParchment,
	'sign-steampunk': SignSteampunk,
	'sign-neon': SignNeon,
	'note-arcane': NoteArcane
};
