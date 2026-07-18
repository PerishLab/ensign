import type { ReactNode } from "react";
import "./Page.scss";

type Props = {
	title: string;
	children: ReactNode;
};

export function Page(props: Props) {
	return (
		<main className="page">
			<h1 className="page-title">{props.title}</h1>
			{props.children}
		</main>
	);
}
