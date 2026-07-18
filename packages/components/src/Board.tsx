import type { ReactNode } from "react";
import "./Board.scss";

type Props = {
	title: string;
	brief?: string;
	children: ReactNode;
};

export function Board(props: Props) {
	return (
		<section className="board">
			<div className="board-head">
				<h2 className="board-title">{props.title}</h2>
				{props.brief ? <p className="board-brief">{props.brief}</p> : null}
			</div>
			<div className="board-body">{props.children}</div>
		</section>
	);
}
