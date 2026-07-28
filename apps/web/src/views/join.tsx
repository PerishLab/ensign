import { Card, Mark } from "@ensign/components";
import { Button, Field, Link, Note, Split } from "@perish/react-components";
import { useState } from "react";
import { back, carry } from "../lib/path";
import { enter, join } from "../lib/wire";

const WARNS = {
	floor: "Passwords need at least 8 characters.",
	code: "That invite code is not open.",
	taken: "That login is already taken.",
	fail: "Something went wrong — try again.",
} as const;

export default function Join() {
	const [code, setCode] = useState("");
	const [login, setLogin] = useState("");
	const [name, setName] = useState("");
	const [pass, setPass] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	async function submit() {
		setBusy(true);
		setWarn("");
		const made = await join(code, login, name, pass);
		if (made === "ok") {
			const entered = await enter(login, pass);
			globalThis.location.assign(
				entered === "ok" ? back() : `/login${carry()}`,
			);
			return;
		}
		setBusy(false);
		setWarn(WARNS[made]);
	}

	return (
		<form
			onSubmit={(event) => {
				event.preventDefault();
				void submit();
			}}
		>
			<Card>
				<Mark note="Join with your invite" />
				<Field label="Invite code" value={code} change={setCode} />
				<Field label="Login" value={login} change={setLogin} />
				<Field label="Name" value={name} change={setName} />
				<Field label="Password" value={pass} change={setPass} kind="password" />
				{warn ? <Note text={warn} tone="warn" /> : null}
				<Button label="Create account" submit wide busy={busy} />
				<Split>
					<Link label="Already aboard? Sign in" href={`/login${carry()}`} />
				</Split>
			</Card>
		</form>
	);
}
