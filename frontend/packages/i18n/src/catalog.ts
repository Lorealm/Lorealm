export type Catalog<T> = {
	[K in keyof T]: T[K] extends string
		? string
		: T[K] extends (...args: infer A) => string
			? (...args: A) => string
			: Catalog<T[K]>;
};

type PluralForms = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };

export function pluralizer(locale: string) {
	const rules = new Intl.PluralRules(locale);
	return (count: number, forms: PluralForms): string => forms[rules.select(count)] ?? forms.other;
}
