import { Bar } from "@ensign/components";
import { Button, Link, Note, Page } from "@perish/react-components";
import { useEffect, useRef, useState } from "react";
import { Actors, Apps, Invites, Teams } from "../lib/boards";
import { Down } from "../lib/down";
import { leave, rows, whoami } from "../lib/wire";

type Row = { id: number } & Record<string, unknown>;

export default function Admin() {
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
			if (who === "fail") {
				setBroke(true);
				return;
			}
			if (typeof who === "string") {
				globalThis.location.assign("/login?return=%2Fadmin");
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
