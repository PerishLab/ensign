import source from "virtual:perish/views";
import { Shell, Views } from "@perish/react-components";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

const root = document.getElementById("root");
if (root !== null) {
	createRoot(root).render(
		<StrictMode>
			<Shell>
				<Views source={source} />
			</Shell>
		</StrictMode>,
	);
}
