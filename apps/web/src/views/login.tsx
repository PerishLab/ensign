import { Card, Mark } from "@ensign/components";
import { Button, Field, Link, Note, Split } from "@perish/react-components";
import { useState } from "react";
import { back, carry } from "../lib/path";
import { say } from "../lib/say";
import { enter } from "../lib/wire";

export default function Login() {
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
		setWarn(
			say(got, {
				miss: "That login and password did not match.",
				barred: "This account is barred.",
			}),
		);
	}

	return (
		<form
			onSubmit={(event) => {
				event.preventDefault();
				void submit();
			}}
		>
			<Card>
				<Mark note="Sign in to continue" />
				<Field label="Login" value={login} change={setLogin} />
				<Field label="Password" value={pass} change={setPass} kind="password" />
				{warn ? <Note text={warn} tone="warn" /> : null}
				<Button label="Sign in" submit wide busy={busy} />
				<Split>
					<Link label="Have an invite?" href={`/join${carry()}`} />
					<Link label="Lost your key?" href={`/recover${carry()}`} />
				</Split>
			</Card>
		</form>
	);
}
