import type { ReactNode } from "react";
import "./Card.scss";

type Props = {
	children: ReactNode;
};

export function Card(props: Props) {
	return (
		<main className="card-page">
			<section className="card">{props.children}</section>
		</main>
	);
}
