import type { IncomingMessage } from "node:http";
import { design } from "@jsr/perish__vite-plugin-design";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const pure = [
	"/logout",
	"/invite",
	"/whoami",
	"/auth",
	"/mint",
	"/repass",
	"/revive",
	"/authorize",
	"/token",
	"/userinfo",
	"/.well-known",
	"/query",
	"/Actor",
	"/Team",
	"/App",
	"/health",
];

const shared = ["/login", "/join"];

function spa(req: IncomingMessage): string | undefined {
	return req.method === "GET" ? "/index.html" : undefined;
}

export default defineConfig(({ command }) => {
	const plugins = [design(), react()];
	if (command !== "serve") {
		return { plugins };
	}
	const target = process.env.ENSIGN_API;
	const raw = process.env.SIDECAR_PORT;
	if (target === undefined || raw === undefined) {
		throw new Error("ensign web dev must be started through sidecar");
	}
	const port = Number(raw);
	if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
		throw new Error(`invalid SIDECAR_PORT: ${raw}`);
	}
	return {
		plugins,
		server: {
			host: "127.0.0.1",
			port,
			strictPort: true,
			proxy: {
				...Object.fromEntries(pure.map((path) => [path, target])),
				...Object.fromEntries(
					shared.map((path) => [path, { target, bypass: spa }]),
				),
			},
		},
	};
});
