import "./Button.scss";

type Props = {
	label: string;
	press: () => void;
	tone?: "solid" | "quiet";
	wide?: boolean;
	busy?: boolean;
};

export function Button(props: Props) {
	const tone = props.tone ?? "solid";
	const rank = props.wide ? "button button-wide" : "button";
	return (
		<button
			className={`${rank} button-${tone}`}
			type="button"
			disabled={props.busy}
			onClick={props.press}
		>
			{props.label}
		</button>
	);
}
