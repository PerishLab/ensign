import type { ReactNode } from "react";
import "./tokens.scss";
import "./themes/light.scss";
import "./themes/dark.scss";
import "./Frame.scss";

type Props = {
	children: ReactNode;
};

export function Frame(props: Props) {
	return <div className="frame">{props.children}</div>;
}
