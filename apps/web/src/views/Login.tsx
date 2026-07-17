import {
	Button,
	Card,
	Field,
	Link,
	Mark,
	Note,
	Split,
} from "@ensign/components";
import { useState } from "react";
import { enter } from "../lib/wire";

export function Login() {
	const [login, setLogin] = useState("");
	const [pass, setPass] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	async function submit() {
		setBusy(true);
		setWarn("");
		const ok = await enter(login, pass);
		setBusy(false);
		if (ok) {
			globalThis.location.assign(back());
		} else {
			setWarn("That login and password did not match.");
		}
	}

	return (
		<Card>
			<Mark note="Sign in to continue" />
			<Field label="Login" value={login} change={setLogin} />
			<Field label="Password" value={pass} change={setPass} kind="password" />
			{warn ? <Note text={warn} tone="warn" /> : null}
			<Button label="Sign in" press={submit} wide busy={busy} />
			<Split>
				<Link label="Have an invite?" href="/join" />
				<Link label="Lost your key?" href="/recover" />
			</Split>
		</Card>
	);
}

function back(): string {
	const seek = new URLSearchParams(globalThis.location.search).get("return");
	return seek?.startsWith("/") ? seek : "/portal";
}
