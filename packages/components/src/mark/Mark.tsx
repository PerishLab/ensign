import "./Mark.scss";

type Props = {
	note?: string;
};

export function Mark(props: Props) {
	return (
		<div className="mark">
			<span className="mark-flag" aria-hidden="true">
				⚑
			</span>
			<span className="mark-name">ensign</span>
			{props.note ? <span className="mark-note">{props.note}</span> : null}
		</div>
	);
}
