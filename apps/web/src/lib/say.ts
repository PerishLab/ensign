const OOPS = "Something went wrong — try again.";

export function say<T extends string>(
	got: T,
	said: Partial<Record<T, string>>,
	oops: string = OOPS,
): string {
	return said[got] ?? oops;
}
