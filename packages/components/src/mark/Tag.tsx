import "./Tag.scss";

type Props = {
	text: string;
	tone?: "calm" | "warn";
};

export function Tag(props: Props) {
	return (
		<span className={`tag tag-${props.tone ?? "calm"}`}>{props.text}</span>
	);
}
