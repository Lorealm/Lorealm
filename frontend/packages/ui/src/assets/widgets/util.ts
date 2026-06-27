export const str = (v: unknown): string => (typeof v === 'string' ? v : v == null ? '' : String(v));
