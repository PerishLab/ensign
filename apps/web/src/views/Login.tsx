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
import { back, carry } from "../lib/path";
import { enter } from "../lib/wire";

export function Login() {
	const [login, setLogin] = useState("");
	const [pass, setPass] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	async function submit() {
		setBusy(true);
		setWarn("");
		const got = await enter(login, pass);
		setBusy(false);
		if (got === "ok") {
			globalThis.location.assign(back());
			return;
		}
		if (got === "miss") {
			setWarn("That login and password did not match.");
		} else if (got === "barred") {
			setWarn("This account is barred.");
		} else {
			setWarn("Something went wrong — try again.");
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
				<Link label="Have an invite?" href={`/join${carry()}`} />
				<Link label="Lost your key?" href={`/recover${carry()}`} />
			</Split>
		</Card>
	);
}
