import { Frame } from "@ensign/components";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Login } from "./views/Login";

const root = document.getElementById("root");
if (root !== null) {
	createRoot(root).render(
		<StrictMode>
			<Frame>
				<Login />
			</Frame>
		</StrictMode>,
	);
}
