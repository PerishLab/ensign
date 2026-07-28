import { Nav } from "@perish/react-components";
import type { ReactNode } from "react";

type Props = {
	children?: ReactNode;
};

export function Bar(props: Props) {
	return (
		<header>
			<Nav>
				<a href="/portal">
					<span aria-hidden="true">⚑</span>
					<span>ensign</span>
				</a>
				{props.children}
			</Nav>
		</header>
	);
}
