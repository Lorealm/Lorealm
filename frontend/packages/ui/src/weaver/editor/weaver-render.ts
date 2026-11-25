export function renderDoc(md: string): string {
	const esc = md.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	const withCode = esc.replace(/`([^`]+)`/g, '<code>$1</code>');
	return withCode
		.split(/\n\n+/)
		.map((p) => `<p>${p.replace(/\n/g, '<br>')}</p>`)
		.join('');
}

export interface ParamDoc {
	name: string;
	description: string;
}

export function infoDom(signature: string, doc: string, params: ParamDoc[] = []): HTMLElement {
	const root = document.createElement('div');
	root.className = 'cm-weaver-tooltip';
	const sig = document.createElement('div');
	sig.className = 'cm-weaver-tooltip-sig';
	sig.textContent = signature;
	root.append(sig);
	if (doc) {
		const body = document.createElement('div');
		body.className = 'cm-weaver-tooltip-doc';
		body.innerHTML = renderDoc(doc);
		root.append(body);
	}
	if (params.length > 0) {
		const list = document.createElement('dl');
		list.className = 'cm-weaver-tooltip-params';
		for (const param of params) {
			const name = document.createElement('dt');
			name.textContent = param.name;
			const desc = document.createElement('dd');
			desc.innerHTML = renderDoc(param.description);
			list.append(name, desc);
		}
		root.append(list);
	}
	return root;
}
