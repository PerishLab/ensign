import { destination, message, submit } from "./protocol.mjs";

export { destination, message, submit } from "./protocol.mjs";

function signed(doc, location, who, target) {
	const leaving = location.pathname === "/logout";
	if (target && !leaving) {
		location.assign(target);
		return true;
	}
	doc.getElementById("identity").textContent = `${who.name} (${who.login})`;
	doc.getElementById("session").hidden = leaving;
	doc.getElementById("logout").hidden = !leaving;
	return false;
}

export async function boot(doc, location, fetcher) {
	const find = (id) => doc.getElementById(id);
	const login = find("login");
	const logout = find("logout");
	const status = find("status");
	const raw = new URLSearchParams(location.search).get("return");
	const target = destination(raw, location.origin);
	const leaving = location.pathname === "/logout";
	find("title").textContent = leaving ? "退出登录" : "登录";
	doc.title = `Ensign · ${leaving ? "退出登录" : "登录"}`;
	find("intro").textContent = leaving
		? "退出后，访问受保护内容时需要重新登录。"
		: "使用你的账号继续。";
	if (raw !== null && target === null)
		status.textContent = "返回地址无效，登录后将停留在此处。";
	try {
		const res = await fetcher("/api/whoami", {
			signal: AbortSignal.timeout(15000),
			credentials: "same-origin",
			cache: "no-store",
			redirect: "error",
		});
		switch (res.status) {
			case 200:
				if (signed(doc, location, await res.json(), target)) return;
				break;
			case 401:
				login.hidden = leaving;
				find("again").hidden = !leaving;
				if (leaving) status.textContent = "会话已结束。";
				break;
			default:
				status.textContent = "暂时无法确认会话状态，你可以尝试重新登录。";
				login.hidden = leaving;
				logout.hidden = !leaving;
				find("again").hidden = !leaving;
		}
	} catch {
		status.textContent = "无法连接服务，请检查网络后重试。";
		logout.hidden = !leaving;
		login.hidden = leaving;
		find("again").hidden = !leaving;
	}
	login.addEventListener("submit", async (event) => {
		event.preventDefault();
		const button = login.querySelector("button");
		button.disabled = true;
		status.textContent = "正在登录…";
		try {
			const res = await submit(fetcher, "/api/login", {
				login: find("handle").value,
				pass: find("password").value,
			});
			if (res.status !== 201) {
				status.textContent = message(res.status);
				return;
			}
			if (target) location.assign(target);
			else location.assign("/login");
		} catch {
			status.textContent = "无法连接服务，请检查网络后重试。";
		} finally {
			find("password").value = "";
			button.disabled = false;
		}
	});
	logout.addEventListener("submit", async (event) => {
		event.preventDefault();
		const button = logout.querySelector("button");
		button.disabled = true;
		try {
			const res = await submit(fetcher, "/api/logout", {});
			if (res.status === 204 || res.status === 401) {
				logout.hidden = true;
				find("session").hidden = true;
				find("again").hidden = false;
				status.textContent =
					res.status === 204 ? "已退出登录。" : "会话已结束。";
			} else status.textContent = message(res.status);
		} catch {
			status.textContent = "无法连接服务，请稍后重试退出。";
		} finally {
			button.disabled = false;
		}
	});
}

if (typeof document !== "undefined") {
	boot(document, window.location, window.fetch.bind(window));
}
