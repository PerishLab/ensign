import { Hero } from "@perish/react-components";

type Props = {
	note?: string;
};

export function Mark(props: Props) {
	return <Hero title="ensign" text={props.note ?? ""} mark="⚑" />;
}
