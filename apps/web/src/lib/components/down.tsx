import { Link, Note, Split } from "@perish/react-components";
import { Card } from "./card";
import { Mark } from "./mark";

export function Down() {
	return (
		<Card>
			<Mark note="Something went wrong" />
			<Note
				text="ensign could not confirm your identity right now. Nothing about your session has changed — try again in a moment."
				tone="warn"
			/>
			<Split>
				<Link label="Try again" href={globalThis.location.pathname} />
				<Link label="Back to sign in" href="/login" />
			</Split>
		</Card>
	);
}
