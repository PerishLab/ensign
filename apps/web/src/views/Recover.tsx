import { Card, Mark } from "@ensign/components";
import { Button, Field, Link, Note, Split } from "@perish/react-components";
import { useState } from "react";
import { carry } from "../lib/path";
import { say } from "../lib/say";
import { enter, revive } from "../lib/wire";

export function Recover() {
	const [login, setLogin] = useState("");
	const [code, setCode] = useState("");
	const [pass, setPass] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	async function submit() {
		setBusy(true);
		setWarn("");
		const got = await revive(login, code, pass);
		if (got === "ok") {
			const entered = await enter(login, pass);
			globalThis.location.assign(entered === "ok" ? "/portal" : "/login");
			return;
		}
		setBusy(false);
		setWarn(
			say(got, {
				floor: "Passwords need at least 8 characters.",
				barred: "This account is barred.",
				miss: "That login and rescue code did not match.",
			}),
		);
	}

	return (
		<Card>
			<Mark note="Recover your account" />
			<Note text="A rescue code works once. Recovering sets a new password, burns all your codes, and ends every session." />
			<Field label="Login" value={login} change={setLogin} />
			<Field label="Rescue code" value={code} change={setCode} />
			<Field
				label="New password"
				value={pass}
				change={setPass}
				kind="password"
			/>
			{warn ? <Note text={warn} tone="warn" /> : null}
			<Button label="Recover" press={submit} wide busy={busy} />
			<Split>
				<Link label="Back to sign in" href={`/login${carry()}`} />
			</Split>
		</Card>
	);
}
