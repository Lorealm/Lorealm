export function blobUrl(avatarUri: string | null | undefined): string | null {
	const hash = avatarUri?.startsWith('sha256:') ? avatarUri.slice('sha256:'.length) : null;
	return hash ? `/api/v1/blobs/${hash}` : null;
}
