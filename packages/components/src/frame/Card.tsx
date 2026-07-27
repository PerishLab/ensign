import { Sheet } from "@perish/react-components";
import type { ReactNode } from "react";

type Props = {
	children: ReactNode;
};

export function Card(props: Props) {
	return (
		<main className="card-page">
			<Sheet>{props.children}</Sheet>
		</main>
	);
}
