import { Frame } from "@ensign/components";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Admin } from "./views/Admin";
import { Join } from "./views/Join";
import { Login } from "./views/Login";
import { Portal } from "./views/Portal";
import { Recover } from "./views/Recover";

function view() {
	switch (globalThis.location.pathname) {
		case "/join":
			return <Join />;
		case "/recover":
			return <Recover />;
		case "/portal":
			return <Portal />;
		case "/admin":
			return <Admin />;
		default:
			return <Login />;
	}
}

const root = document.getElementById("root");
if (root !== null) {
	createRoot(root).render(
		<StrictMode>
			<Frame>{view()}</Frame>
		</StrictMode>,
	);
}
