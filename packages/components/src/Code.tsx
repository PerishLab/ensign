import "./Code.scss";

type Props = {
	text: string;
};

export function Code(props: Props) {
	return <code className="code">{props.text}</code>;
}
