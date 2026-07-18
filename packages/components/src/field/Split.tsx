import type { ReactNode } from "react";
import "./Split.scss";

type Props = {
	children: ReactNode;
};

export function Split(props: Props) {
	return <div className="split">{props.children}</div>;
}
