import "./Link.scss";

type Props = {
	label: string;
	href: string;
};

export function Link(props: Props) {
	return (
		<a className="link" href={props.href}>
			{props.label}
		</a>
	);
}
