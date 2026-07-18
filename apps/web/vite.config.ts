import type { IncomingMessage } from "node:http";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const target = "http://127.0.0.1:3500";

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

export default defineConfig({
	plugins: [react()],
	server: {
		host: "127.0.0.1",
		port: 5173,
		strictPort: true,
		proxy: {
			...Object.fromEntries(pure.map((path) => [path, target])),
			...Object.fromEntries(
				shared.map((path) => [path, { target, bypass: spa }]),
			),
		},
	},
});
