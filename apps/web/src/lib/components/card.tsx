import { Frame, Sheet } from "@perish/react-components";
import type { ReactNode } from "react";

type Props = {
	children: ReactNode;
};

export function Card(props: Props) {
	return (
		<main>
			<Frame>
				<Sheet>{props.children}</Sheet>
			</Frame>
		</main>
	);
}
