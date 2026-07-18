import {
	Bar,
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
} from "@ensign/components";
import { useEffect, useRef, useState } from "react";
import { useHush } from "../lib/hush";
import { invite, leave, put, rows, set, whoami } from "../lib/wire";
import { Down } from "./Down";

type Row = { id: number } & Record<string, unknown>;

export function Admin() {
	const [ready, setReady] = useState(false);
	const [broke, setBroke] = useState(false);
	const [me, setMe] = useState(0);
	const [warn, setWarn] = useState("");
	const [actors, setActors] = useState<Row[] | null>([]);
	const [teams, setTeams] = useState<Row[] | null>([]);
	const [apps, setApps] = useState<Row[] | null>([]);

	const stamp = useRef(0);

	async function load() {
		const turn = ++stamp.current;
		const [held, crews, kin] = await Promise.all([
			rows("Actor"),
			rows("Team"),
			rows("App"),
		]);
		if (turn !== stamp.current) {
			return;
		}
		setActors(held);
		setTeams(crews);
		setApps(kin);
	}

	useEffect(() => {
		whoami().then((who) => {
			if (who === "anon") {
				globalThis.location.assign("/login?return=%2Fadmin");
				return;
			}
			if (who === "fail") {
				setBroke(true);
				return;
			}
			setMe(who.id);
			setReady(true);
			load();
		});
	}, []);

	async function out() {
		if (await leave()) {
			globalThis.location.replace("/login");
		} else {
			setWarn("Sign-out failed — your session may still be live.");
		}
	}

	if (broke) {
		return <Down />;
	}
	if (!ready) {
		return null;
	}
	return (
		<>
			<Bar>
				<Link label="Portal" href="/portal" />
				<Button label="Sign out" press={out} tone="quiet" />
			</Bar>
			<Page title="Admin">
				{warn ? <Note text={warn} tone="warn" /> : null}
				<Invites />
				<Actors actors={actors} me={me} reload={load} />
				<Teams teams={teams} reload={load} />
				<Apps apps={apps} reload={load} />
			</Page>
		</>
	);
}

function Invites() {
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
		if (fresh.shut === "anon") {
			setWarn("Your session ended — sign in again.");
		} else if (fresh.shut === "denied") {
			setWarn("Refused. Opening invites takes an invite grant.");
		} else if (fresh.shut === "fail") {
			setWarn("Something went wrong — try again.");
		} else {
			setCode(fresh.code);
			setNote("");
		}
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
					<Code text={code} />
				</>
			) : null}
			{warn ? <Note text={warn} tone="warn" /> : null}
		</Board>
	);
}

function Actors(props: {
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
		} else if (got === "anon") {
			setWarn("Your session ended — sign in again.");
		} else if (got === "denied") {
			setWarn("Refused. Barring takes a set grant on Actor.");
		} else {
			setWarn("Something went wrong — try again.");
		}
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

function Teams(props: { teams: Row[] | null; reload: () => Promise<void> }) {
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
		} else if (made === "clash") {
			setWarn("That team name is taken.");
		} else if (made === "anon") {
			setWarn("Your session ended — sign in again.");
		} else if (made === "denied") {
			setWarn("Refused. Founding teams takes a put grant on Team.");
		} else {
			setWarn("Something went wrong — try again.");
		}
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

function Apps(props: { apps: Row[] | null; reload: () => Promise<void> }) {
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
		} else if (made === "clash") {
			setWarn("That slug is taken.");
		} else if (made === "anon") {
			setWarn("Your session ended — sign in again.");
		} else if (made === "denied") {
			setWarn("Refused. Registering takes a put grant on App.");
		} else {
			setWarn("Invalid or failed — check the fields and try again.");
		}
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
