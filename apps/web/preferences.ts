export type Scope = "app" | "shared";
type Declaration = { state: "missing" } | { state: "present"; value: string };
export type Item = {
	name: string;
	options: string[];
	shared: Declaration;
	app: Declaration;
	effective: string;
	source: "default" | "shared" | "app" | "policy";
	editable: boolean;
};
export type State = {
	revision: string;
	app: string | null;
	items: Item[];
	presentation: { system: string; tone: "light" | "dark"; basis: string };
};
export type Views = { app: State; shared: State };
type Context = {
	access: string;
	tone: "light" | "dark";
	fetcher?: typeof fetch;
};
type Change = {
	op: "set" | "reset";
	scope: { kind: Scope; name?: string };
	name: string;
	value?: string;
};

export class Failure extends Error {
	status: number;
	constructor(status: number) {
		super(
			status === 409
				? "设置已在其他页面更新，请重新读取后重试。"
				: status === 401
					? "身份确认已过期，请重新读取后重试。"
					: status === 403
						? "当前无法修改此设置。"
						: "暂时无法完成操作，请重新读取设置后重试。",
		);
		this.status = status;
	}
}

function address(scope: Scope, tone: string): string {
	return `/folio/v1/me/settings?${scope === "app" ? "app=ensign&" : ""}tone=${tone}`;
}

async function request(
	context: Context,
	scope: Scope,
	body?: { revision: string; changes: Change[] },
): Promise<State> {
	const response = await (context.fetcher ?? fetch)(
		address(scope, context.tone),
		{
			method: body ? "PATCH" : "GET",
			cache: "no-store",
			credentials: "omit",
			redirect: "error",
			signal: AbortSignal.timeout(15000),
			headers: {
				authorization: `Bearer ${context.access}`,
				"content-type": "application/json",
			},
			body: body ? JSON.stringify(body) : undefined,
		},
	);
	if (!response.ok) throw new Failure(response.status);
	return response.json();
}

export async function read(context: Context): Promise<Views> {
	const [app, shared] = await Promise.all([
		request(context, "app"),
		request(context, "shared"),
	]);
	return { app, shared };
}

export async function write(
	context: Context,
	state: State,
	change: Change,
): Promise<Views> {
	await request(context, change.scope.kind, {
		revision: state.revision,
		changes: [change],
	});
	return read(context);
}

export function intent(scope: Scope, name: string, value?: string): Change {
	return {
		op: value === undefined ? "reset" : "set",
		scope: { kind: scope, ...(scope === "app" ? { name: "ensign" } : {}) },
		name,
		...(value === undefined ? {} : { value }),
	};
}
