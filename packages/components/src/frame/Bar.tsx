import type { ReactNode } from "react";

type Props = {
	children?: ReactNode;
};

export function Bar(props: Props) {
	return (
		<header className="bar">
			<a className="bar-mark" href="/portal">
				<span className="bar-flag" aria-hidden="true">
					⚑
				</span>
				<span className="bar-name">ensign</span>
			</a>
			<nav className="bar-side">{props.children}</nav>
		</header>
	);
}
