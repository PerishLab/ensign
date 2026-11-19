export function destination(raw, origin) {
	if (typeof raw !== "string" || !raw.startsWith("/") || raw.startsWith("//"))
		return null;
	if (
		raw.includes("\\") ||
		[...raw].some(
			(char) => char.codePointAt(0) < 32 || char.codePointAt(0) === 127,
		)
	)
		return null;
	const path = raw.split(/[?#]/u, 1)[0];
	if (path.includes("%")) return null;
	try {
		const url = new URL(raw, origin);
		if (url.origin !== origin || ["/login", "/logout"].includes(url.pathname))
			return null;
		return `${url.pathname}${url.search}${url.hash}`;
	} catch {
		return null;
	}
}

export function message(status) {
	if (status === 401 || status === 403) return "账号或密码不正确。";
	if (status === 429) return "请求过于频繁，请稍后重试。";
	return "暂时无法完成请求，请稍后重试。";
}

export async function submit(fetcher, path, fields) {
	return fetcher(path, {
		method: "POST",
		credentials: "same-origin",
		cache: "no-store",
		redirect: "error",
		signal: AbortSignal.timeout(15000),
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(fields),
	});
}
