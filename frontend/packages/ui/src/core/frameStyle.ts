/** One of the app's font faces, as bytes a frame can build it from. */
export interface FrameFont {
	url: string;
	family: string;
	descriptors: FontFaceDescriptors;
	buffer: ArrayBuffer;
}

/** Every custom property the app's stylesheets declare on `:root` */
export function rootVariables(): string {
	const names = new Set<string>();
	for (const rule of cssRules()) {
		if (rule instanceof CSSStyleRule && rule.selectorText === ':root') {
			for (let i = 0; i < rule.style.length; i++) {
				if (rule.style[i].startsWith('--')) names.add(rule.style[i]);
			}
		}
	}

	const computed = getComputedStyle(document.documentElement);
	const declarations = [...names]
		.map((name) => [name, computed.getPropertyValue(name).trim()])
		// Nothing that could end the `<style>` or rule it's written into.
		.filter(([, value]) => value !== '' && !/[<{}]/.test(value))
		.map(([name, value]) => `${name}: ${value};`)
		.join(' ');
	return `:root { ${declarations} }`;
}

/**
 * The app's font faces that have loaded, with their bytes. Bypasses a frame's CSP.
 */
export async function loadedFonts(): Promise<FrameFont[]> {
	const loaded: FontFace[] = [];
	document.fonts.forEach((face) => {
		if (face.status === 'loaded') loaded.push(face);
	});

	const faces = fontFaceRules().filter((rule) => loaded.some((face) => sameFace(face, rule)));
	const fonts = await Promise.all(
		faces.map(async (face) => {
			try {
				return { ...face, buffer: await fetchFont(face.url) };
			} catch {
				return null;
			}
		})
	);
	return fonts.filter((font) => font !== null);
}

type FaceRule = Omit<FrameFont, 'buffer'>;

/** Every rule in the document's readable stylesheets, nested ones included. */
function cssRules(): CSSRule[] {
	const out: CSSRule[] = [];
	const collect = (rules: CSSRuleList) => {
		for (const rule of Array.from(rules)) {
			out.push(rule);
			if ('cssRules' in rule) collect((rule as CSSGroupingRule).cssRules);
		}
	};
	for (const sheet of Array.from(document.styleSheets)) {
		try {
			collect(sheet.cssRules);
		} catch {
			// A cross-origin stylesheet; it can't be read, and declares nothing of ours.
		}
	}
	return out;
}

function fontFaceRules(): FaceRule[] {
	return cssRules().flatMap((rule) => {
		if (!(rule instanceof CSSFontFaceRule)) return [];
		const src = /url\((['"]?)(.*?)\1\)/.exec(rule.style.getPropertyValue('src'));
		if (!src) return [];
		const base = rule.parentStyleSheet?.href ?? document.baseURI;
		const get = (name: string) => rule.style.getPropertyValue(name).trim() || undefined;
		return [
			{
				url: new URL(src[2], base).href,
				family: unquote(rule.style.getPropertyValue('font-family')),
				descriptors: {
					weight: get('font-weight'),
					style: get('font-style'),
					stretch: get('font-stretch'),
					unicodeRange: get('unicode-range')
				}
			}
		];
	});
}

function sameFace(face: FontFace, rule: FaceRule): boolean {
	return (
		unquote(face.family) === rule.family &&
		face.style === (rule.descriptors.style ?? 'normal') &&
		ranges(face.unicodeRange) === ranges(rule.descriptors.unicodeRange ?? '')
	);
}

function unquote(family: string): string {
	return family.trim().replace(/^(['"])(.*)\1$/, '$2');
}

/** A `unicode-range` in one spelling, however it was written. */
function ranges(range: string): string {
	return (range || 'U+0-10FFFF')
		.toUpperCase()
		.replace(/\s/g, '')
		.split(',')
		.map((part) => {
			const m = /^U\+([0-9A-F?]+)(?:-([0-9A-F]+))?$/.exec(part);
			if (!m) return part;
			const start = parseInt(m[1].replace(/\?/g, '0'), 16);
			const end = parseInt(m[2] ?? m[1].replace(/\?/g, 'F'), 16);
			return `${start}-${end}`;
		})
		.join(',');
}

const fontCache = new Map<string, Promise<ArrayBuffer>>();

function fetchFont(url: string): Promise<ArrayBuffer> {
	let pending = fontCache.get(url);
	if (!pending) {
		pending = fetch(url).then((res) => {
			if (!res.ok) throw new Error(`${res.status}`);
			return res.arrayBuffer();
		});
		pending.catch(() => fontCache.delete(url));
		fontCache.set(url, pending);
	}
	return pending;
}
