export async function enter(login: string, pass: string): Promise<boolean> {
	const res = await fetch("/login", {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ login, pass }),
	});
	return res.ok;
}
