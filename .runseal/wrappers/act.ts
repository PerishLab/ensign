import { cli, flags } from "@/lib/cli.ts";
import { bin } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const host = "127.0.0.1";
const port = 13500;
const base = `http://${host}:${port}`;

function usage(): void {
  io.print("Usage: runseal :act");
  io.print("");
  io.print("Ensign stage acts over the running binary (docs/spec.md).");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
if (flags(args).help()) {
  flags(args).positionals("act", { allowHelp: true });
  usage();
  Deno.exit(0);
}
flags(args).positionals("act");

const root = await bin("git").text(["rev-parse", "--show-toplevel"]);
const dir = `${root}/.local/act`;
await Deno.mkdir(dir, { recursive: true });
await Deno.writeTextFile(
  `${dir}/keel.toml`,
  `[listen]\nhost = "${host}"\nport = ${port}\nprefix = ""\n\n[store]\nkind = "memory"\n\n[identity]\nunit = "Actor"\n\n[cache]\nkind = "memory"\n`,
);

io.print("==> build api");
await bin("cargo").run(["build", "--locked"], { cwd: root });

io.print(`==> boot api on ${base}`);
const pg = Deno.env.get("KEEL_PG");
const env: Record<string, string> = pg ? { KEEL_PG: pg, KEEL_FRESH: "1" } : {};
if (pg) {
  io.print("==> store: postgres");
}
const child = new Deno.Command("cargo", {
  args: ["run", "--locked", "-p", "api", "--", dir],
  cwd: root,
  env,
  stdin: "null",
  stdout: "null",
  stderr: "piped",
}).spawn();

let boot = "";
const drain = (async () => {
  const decoder = new TextDecoder();
  for await (const part of child.stderr) {
    boot += decoder.decode(part);
  }
})();

let failed = false;
try {
  await ready(`${base}/health`, 40);
  const crown = { authorization: `sudo ${await sudo()}` };

  io.print("==> act 1: identity floor");
  await check("possession admits exactly once", async () => {
    const seal = await digest("golden");
    const card = await post("/invite", { hash: seal, note: "first" }, crown);
    if (card.status !== 201) {
      throw new Error(`invite ${card.status}`);
    }
    const made = await post("/join", {
      code: "golden",
      login: "ada",
      name: "Ada",
      pass: "seaworthy",
    });
    if (made.status !== 201) {
      throw new Error(`join ${made.status}`);
    }
    const again = await post("/join", {
      code: "golden",
      login: "bob",
      name: "Bob",
      pass: "drift",
    });
    if (again.status !== 404) {
      throw new Error(`burned invite ${again.status}`);
    }
  });

  let jar = "";
  await check("the password floor holds", async () => {
    const wrong = await post("/login", { login: "ada", pass: "adrift" });
    if (wrong.status !== 401) {
      throw new Error(`wrong pass ${wrong.status}`);
    }
    const right = await post("/login", { login: "ada", pass: "seaworthy" });
    if (right.status !== 201) {
      throw new Error(`login ${right.status}`);
    }
    jar = cookie(right);
    const me = await get("/whoami", { cookie: jar });
    if (me.status !== 200 || (me.body as { login?: string }).login !== "ada") {
      throw new Error(`whoami ${me.status}`);
    }
    const anon = await get("/whoami");
    if (anon.status !== 401) {
      throw new Error(`anon whoami ${anon.status}`);
    }
  });

  await check("a rescue code burns once", async () => {
    const minted = await post("/mint", {}, { cookie: jar });
    const codes = (minted.body as { codes?: string[] }).codes ?? [];
    if (minted.status !== 201 || codes.length !== 3) {
      throw new Error(`mint ${minted.status}`);
    }
    const back = await post("/revive", { login: "ada", code: codes[0] });
    if (back.status !== 201) {
      throw new Error(`revive ${back.status}`);
    }
    const spent = await post("/revive", { login: "ada", code: codes[0] });
    if (spent.status !== 401) {
      throw new Error(`spent code ${spent.status}`);
    }
  });

  await check("logout ends the session", async () => {
    const out = await post("/logout", {}, { cookie: jar });
    if (out.status !== 204) {
      throw new Error(`logout ${out.status}`);
    }
    const gone = await get("/whoami", { cookie: jar });
    if (gone.status !== 401) {
      throw new Error(`dead session ${gone.status}`);
    }
  });

  io.print("act: clean");
} catch (err) {
  failed = true;
  io.print(`act: ${err instanceof Error ? err.message : String(err)}`);
} finally {
  child.kill("SIGKILL");
  await drain.catch(() => {});
}
if (failed) {
  io.print(boot.split("\n").slice(-20).join("\n"));
  Deno.exit(1);
}

type Reply = { status: number; body: unknown; headers: Headers };

async function post(
  path: string,
  body: Record<string, unknown>,
  head: Record<string, string> = {},
): Promise<Reply> {
  const res = await fetch(`${base}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...head },
    body: JSON.stringify(body),
  });
  return { status: res.status, body: await parse(res), headers: res.headers };
}

async function get(
  path: string,
  head: Record<string, string> = {},
): Promise<Reply> {
  const res = await fetch(`${base}${path}`, { headers: head });
  return { status: res.status, body: await parse(res), headers: res.headers };
}

async function parse(res: Response): Promise<unknown> {
  const text = await res.text();
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function cookie(reply: Reply): string {
  const jar = reply.headers.get("set-cookie") ?? "";
  const hit = jar.match(/session=[0-9a-f]+/);
  if (!hit) {
    throw new Error("no session cookie");
  }
  return hit[0];
}

async function digest(code: string): Promise<string> {
  const raw = new TextEncoder().encode(code);
  const sum = await crypto.subtle.digest("SHA-256", raw);
  return Array.from(new Uint8Array(sum))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

async function sudo(): Promise<string> {
  for (let i = 0; i < 40; i++) {
    const hit = boot.match(/sudo token ([0-9a-f]+)/);
    if (hit) {
      return hit[1];
    }
    await sleep(250);
  }
  throw new Error("no sudo token in boot log");
}

async function ready(url: string, tries: number): Promise<void> {
  for (let i = 0; i < tries; i++) {
    try {
      const res = await fetch(url);
      await res.body?.cancel();
      if (res.ok) {
        return;
      }
    } catch { /* retry */ }
    await sleep(250);
  }
  throw new Error(`timeout ${url}`);
}

async function check(label: string, run: () => Promise<void>): Promise<void> {
  io.print(`==> ${label}`);
  await run();
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
