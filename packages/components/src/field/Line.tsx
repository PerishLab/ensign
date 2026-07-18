import type { ReactNode } from "react";
import "./Line.scss";

type Props = {
	name: string;
	meta?: string;
	children?: ReactNode;
};

export function Line(props: Props) {
	return (
		<div className="line">
			<div className="line-text">
				<span className="line-name">{props.name}</span>
				{props.meta ? <span className="line-meta">{props.meta}</span> : null}
			</div>
			{props.children ? (
				<div className="line-side">{props.children}</div>
			) : null}
		</div>
	);
}
