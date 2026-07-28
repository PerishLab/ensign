import { Bar } from "@ensign/components";
import {
	Board,
	Button,
	Code,
	Field,
	Line,
	Link,
	Note,
	Page,
	Split,
	Tag,
} from "@perish/react-components";
import { useEffect, useState } from "react";
import { Down } from "../lib/down";
import { useHush } from "../lib/hush";
import { say } from "../lib/say";
import { crews, leave, mint, repass, whoami } from "../lib/wire";

type Me = { id: number; login: string; name: string };

function Guard() {
	const [old, setOld] = useState("");
	const [pass, setPass] = useState("");
	const [note, setNote] = useState("");
	const [tone, setTone] = useState<"calm" | "warn">("calm");
	const [busy, setBusy] = useState(false);

	async function shift() {
		setBusy(true);
		setNote("");
		const got = await repass(old, pass);
		setBusy(false);
		if (got === "ok") {
			setOld("");
			setPass("");
			setTone("calm");
			setNote("Password set.");
			return;
		}
		setTone("warn");
		setNote(
			say(got, {
				floor: "Passwords need at least 8 characters.",
				miss: "That current password is wrong.",
				anon: "Your session ended — sign in again.",
			}),
		);
	}

	return (
		<Board
			title="Password"
			brief="Your brain key. Changing it takes your current password."
		>
			<form
				onSubmit={(event) => {
					event.preventDefault();
					void shift();
				}}
			>
				<Field
					label="Current password"
					value={old}
					change={setOld}
					kind="password"
				/>
				<Split>
					<Field
						label="New password"
						value={pass}
						change={setPass}
						kind="password"
					/>
					<Button label="Set password" submit tone="quiet" busy={busy} />
				</Split>
			</form>
			{note ? <Note text={note} tone={tone} /> : null}
		</Board>
	);
}

export default function Portal() {
	const [me, setMe] = useState<Me | null>(null);
	const [broke, setBroke] = useState(false);
	const [teams, setTeams] = useState<string[]>([]);
	const [codes, setCodes] = useState<string[]>([]);
	const [warn, setWarn] = useState("");
	const [fault, setFault] = useState("");
	const [busy, setBusy] = useState(false);

	useHush(() => setCodes([]));

	useEffect(() => {
		whoami().then(async (who) => {
			if (who === "fail") {
				setBroke(true);
				return;
			}
			if (typeof who === "string") {
				globalThis.location.assign("/login?return=%2Fportal");
				return;
			}
			setMe(who);
			const held = await crews(who.id);
			if (held === null) {
				setWarn("Could not load your teams.");
			} else {
				setTeams(held);
			}
		});
	}, []);

	async function remint() {
		setBusy(true);
		setFault("");
		const fresh = await mint();
		setBusy(false);
		if (Array.isArray(fresh)) {
			setCodes(fresh);
			return;
		}
		setFault(
			say(fresh, {
				anon: "Your session ended — sign in again.",
				fail: "Minting failed — try again.",
			}),
		);
	}

	async function out() {
		if (await leave()) {
			setCodes([]);
			globalThis.location.replace("/login");
		} else {
			setWarn("Sign-out failed — your session may still be live.");
		}
	}

	if (broke) {
		return <Down />;
	}
	if (me === null) {
		return null;
	}
	return (
		<>
			<Bar>
				<Link label="Admin" href="/admin" />
				<Button label="Sign out" press={out} tone="quiet" />
			</Bar>
			<Page title={`Welcome, ${me.name || me.login}`}>
				{warn ? <Note text={warn} tone="warn" /> : null}
				<Board title="Identity" brief="What ensign asserts about you.">
					<Line name={me.login} meta={`operator ${me.id}`}>
						{teams.map((team) => (
							<Tag key={team} text={team} />
						))}
					</Line>
				</Board>
				<Guard />
				<Board
					title="Rescue codes"
					brief="Your paper key for a lost password. Any one code recovers the account once — recovering sets a new password and burns the whole set. Mint replaces every code."
				>
					{codes.length > 0 ? (
						<>
							<Note
								text="Write these down now — they are shown only once."
								tone="warn"
							/>
							{codes.map((code) => (
								<Code key={code}>{code}</Code>
							))}
						</>
					) : (
						<Line name="A fresh set of one-time codes, shown once.">
							<Button
								label="Mint new codes"
								press={remint}
								tone="quiet"
								busy={busy}
							/>
						</Line>
					)}
					{fault ? <Note text={fault} tone="warn" /> : null}
				</Board>
			</Page>
		</>
	);
}
