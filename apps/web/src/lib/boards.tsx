import {
	Board,
	Button,
	Code,
	Field,
	Line,
	Note,
	Split,
	Tag,
} from "@perish/react-components";
import { useState } from "react";
import { useHush } from "./hush";
import { say } from "./say";
import { invite, put, set } from "./wire";

type Row = { id: number } & Record<string, unknown>;

export function Invites() {
	const [note, setNote] = useState("");
	const [code, setCode] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	useHush(() => setCode(""));

	async function open() {
		setBusy(true);
		setWarn("");
		const fresh = await invite(note);
		setBusy(false);
		if (fresh.shut === "") {
			setCode(fresh.code);
			setNote("");
			return;
		}
		setWarn(
			say(fresh.shut, {
				anon: "Your session ended — sign in again.",
				denied: "Refused. Opening invites takes an invite grant.",
			}),
		);
	}

	return (
		<Board
			title="Invites"
			brief="Joining is invitation-only. An invite code admits exactly one person."
		>
			<Split>
				<Field
					label="Note"
					value={note}
					change={setNote}
					hint="who it is for"
				/>
				<Button label="Open invite" press={open} busy={busy} />
			</Split>
			{code ? (
				<>
					<Note
						text="Hand this code over now — it is shown only once."
						tone="warn"
					/>
					<Code>{code}</Code>
				</>
			) : null}
			{warn ? <Note text={warn} tone="warn" /> : null}
		</Board>
	);
}

export function Actors(props: {
	actors: Row[] | null;
	me: number;
	reload: () => Promise<void>;
}) {
	const [warn, setWarn] = useState("");

	async function flip(row: Row) {
		setWarn("");
		const next = row.barred === true ? "false" : "true";
		const got = await set("Actor", row.id, { barred: next });
		if (got === "ok") {
			await props.reload();
			return;
		}
		setWarn(
			say(got, {
				anon: "Your session ended — sign in again.",
				denied: "Refused. Barring takes a set grant on Actor.",
			}),
		);
	}

	if (props.actors === null) {
		return (
			<Board title="Actors" brief="Everyone ensign answers for.">
				<Note text="Could not load actors." tone="warn" />
			</Board>
		);
	}
	return (
		<Board title="Actors" brief="Everyone ensign answers for.">
			{props.actors.map((row) => (
				<Line
					key={row.id}
					name={String(row.login)}
					meta={String(row.kind ?? "")}
				>
					{row.barred === true ? <Tag text="barred" tone="warn" /> : null}
					{row.kind === "svc" || row.id === props.me ? null : (
						<Button
							label={row.barred === true ? "Unbar" : "Bar"}
							press={() => flip(row)}
							tone="quiet"
						/>
					)}
				</Line>
			))}
			{warn ? <Note text={warn} tone="warn" /> : null}
		</Board>
	);
}

export function Teams(props: {
	teams: Row[] | null;
	reload: () => Promise<void>;
}) {
	const [name, setName] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	async function found() {
		setBusy(true);
		setWarn("");
		const made = await put("Team", { name });
		setBusy(false);
		if (made === "ok") {
			setName("");
			await props.reload();
			return;
		}
		setWarn(
			say(made, {
				clash: "That team name is taken.",
				anon: "Your session ended — sign in again.",
				denied: "Refused. Founding teams takes a put grant on Team.",
			}),
		);
	}

	if (props.teams === null) {
		return (
			<Board title="Teams" brief="Groups that ride into tokens as team claims.">
				<Note text="Could not load teams." tone="warn" />
			</Board>
		);
	}
	return (
		<Board title="Teams" brief="Groups that ride into tokens as team claims.">
			{props.teams.map((row) => (
				<Line key={row.id} name={String(row.name)} />
			))}
			<Split>
				<Field label="New team" value={name} change={setName} />
				<Button label="Found" press={found} tone="quiet" busy={busy} />
			</Split>
			{warn ? <Note text={warn} tone="warn" /> : null}
		</Board>
	);
}

export function Apps(props: {
	apps: Row[] | null;
	reload: () => Promise<void>;
}) {
	const [name, setName] = useState("");
	const [slug, setSlug] = useState("");
	const [home, setHome] = useState("");
	const [redirect, setRedirect] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	async function register() {
		setBusy(true);
		setWarn("");
		const made = await put("App", {
			name,
			slug,
			home,
			redirect,
			secret: "",
			mode: "oidc",
		});
		setBusy(false);
		if (made === "ok") {
			setName("");
			setSlug("");
			setHome("");
			setRedirect("");
			await props.reload();
			return;
		}
		setWarn(
			say(
				made,
				{
					clash: "That slug is taken.",
					anon: "Your session ended — sign in again.",
					denied: "Refused. Registering takes a put grant on App.",
				},
				"Invalid or failed — check the fields and try again.",
			),
		);
	}

	if (props.apps === null) {
		return (
			<Board title="Apps" brief="Relying parties that sign in through ensign.">
				<Note text="Could not load apps." tone="warn" />
			</Board>
		);
	}
	return (
		<Board title="Apps" brief="Relying parties that sign in through ensign.">
			{props.apps.map((row) => (
				<Line
					key={row.id}
					name={String(row.name)}
					meta={String(row.slug ?? "")}
				>
					<Tag text={String(row.mode ?? "")} />
				</Line>
			))}
			<Field label="Name" value={name} change={setName} />
			<Field label="Slug" value={slug} change={setSlug} />
			<Field label="Home URL" value={home} change={setHome} />
			<Field label="Redirect URL" value={redirect} change={setRedirect} />
			<Button label="Register app" press={register} tone="quiet" busy={busy} />
			{warn ? <Note text={warn} tone="warn" /> : null}
		</Board>
	);
}
