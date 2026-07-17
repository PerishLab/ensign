import "./Field.scss";

type Props = {
	label: string;
	value: string;
	change: (next: string) => void;
	kind?: "text" | "password";
	hint?: string;
};

export function Field(props: Props) {
	return (
		<label className="field">
			<span className="field-label">{props.label}</span>
			<input
				className="field-input"
				type={props.kind ?? "text"}
				value={props.value}
				placeholder={props.hint}
				autoComplete="off"
				onChange={(event) => props.change(event.target.value)}
			/>
		</label>
	);
}
