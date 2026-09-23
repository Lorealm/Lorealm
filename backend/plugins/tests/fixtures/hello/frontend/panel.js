// An unsafe slot: runs in the client itself, with the same API a frame gets.
export function mount(element, api) {
	const button = document.createElement('button');
	const out = document.createElement('p');
	button.textContent = 'Greet';
	button.onclick = async () => {
		try {
			out.textContent = await api.call('greet', { name: 'Ada' });
		} catch (e) {
			out.textContent = `Failed: ${e.message}`;
		}
	};
	element.append(button, out);
	return () => element.replaceChildren();
}
