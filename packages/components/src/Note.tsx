import "./Note.scss";

type Props = {
	text: string;
	tone?: "warn" | "calm";
};

export function Note(props: Props) {
	return <p className={`note note-${props.tone ?? "calm"}`}>{props.text}</p>;
}
