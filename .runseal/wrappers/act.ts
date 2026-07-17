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

  io.print("==> act 2: directory");
  const boss = await enrol(crown, "boss");
  const hand = await enrol(crown, "hand");
  const guest = await enrol(crown, "guest");
  await grant(crown, { who: boss.id, verb: "*", unit: "App", scope: "all" });
  await grant(crown, { who: boss.id, verb: "*", unit: "Team", scope: "all" });

  await check("admin registers an app, stranger refused", async () => {
    const made = await post("/App", {
      name: "Wiki",
      slug: "wiki",
      home: "https://wiki.perish.top",
      redirect: "https://wiki.perish.top/callback",
      secret: "",
      mode: "oidc",
    }, boss.head);
    if (made.status !== 201) {
      throw new Error(`register ${made.status}`);
    }
    const seen = await get("/App", hand.head);
    const rows = seen.body as unknown[];
    if (!Array.isArray(rows) || rows.length !== 1) {
      throw new Error(`member sees ${JSON.stringify(seen.body)}`);
    }
    const barred = await post("/App", {
      name: "Rogue",
      slug: "rogue",
      home: "https://x",
      redirect: "https://x/cb",
      secret: "",
      mode: "oidc",
    }, guest.head);
    if (barred.status !== 403) {
      throw new Error(`stranger app ${barred.status}`);
    }
  });

  await check("a team grant admits and revokes", async () => {
    const team = await post("/Team", { name: "editors" }, boss.head);
    const crew = (team.body as { id: number }).id;
    await grant(crown, {
      who: `Team ${crew}`,
      verb: "put",
      unit: "App",
      scope: "all",
    });
    const before = await post("/App", app("one"), hand.head);
    if (before.status !== 403) {
      throw new Error(`pre-member write ${before.status}`);
    }
    const tie = await post(
      `/Team/${crew}/members`,
      { right: hand.id },
      boss.head,
    );
    if (tie.status !== 201) {
      throw new Error(`enrol ${tie.status}`);
    }
    const during = await post("/App", app("two"), hand.head);
    if (during.status !== 201) {
      throw new Error(`member write ${during.status}`);
    }
    const bond = (tie.body as { id: number }).id;
    const cut = await drop(`/Team/${crew}/members/${bond}`, boss.head);
    if (cut !== 204) {
      throw new Error(`revoke ${cut}`);
    }
    const after = await post("/App", app("three"), hand.head);
    if (after.status !== 403) {
      throw new Error(`access outlived membership ${after.status}`);
    }
  });

  io.print("==> act 3: oidc provider");
  const disc = await get("/.well-known/openid-configuration");
  const meta = disc.body as Record<string, unknown>;
  await check("discovery advertises the code flow", () => {
    if (meta.issuer !== base) {
      throw new Error(`issuer ${meta.issuer}`);
    }
    const methods = meta.code_challenge_methods_supported as string[];
    if (!methods.includes("S256")) {
      throw new Error("no S256");
    }
    return Promise.resolve();
  });

  await post("/App", {
    name: "Portal",
    slug: "portal",
    home: "http://127.0.0.1:9999/",
    redirect: "http://127.0.0.1:9999/cb",
    secret: "",
    mode: "oidc",
  }, crown);
  const sailor = await enrol(crown, "sailor");

  let renew = "";
  await check("code+PKCE mints a verifiable id token", async () => {
    const pk = await pkce();
    const code = await dance(sailor.head, pk.challenge);
    const grip = await form("/token", {
      grant_type: "authorization_code",
      code,
      redirect_uri: "http://127.0.0.1:9999/cb",
      client_id: "portal",
      code_verifier: pk.verifier,
    });
    const bag = grip.body as Record<string, string>;
    if (grip.status !== 200 || !bag.id_token) {
      throw new Error(`token ${grip.status}`);
    }
    const claims = await attest(bag.id_token);
    if (claims.iss !== base || claims.aud !== "portal") {
      throw new Error(`claims ${JSON.stringify(claims)}`);
    }
    if (claims.sub !== String(sailor.id) || claims.login !== "sailor") {
      throw new Error(`subject ${JSON.stringify(claims)}`);
    }
    const seen = await get("/userinfo", {
      authorization: `Bearer ${bag.access_token}`,
    });
    if ((seen.body as { sub?: string }).sub !== String(sailor.id)) {
      throw new Error(`userinfo ${JSON.stringify(seen.body)}`);
    }
    renew = bag.refresh_token;
  });

  await check("refresh rotates and retires the old token", async () => {
    const fresh = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: renew,
    });
    if (fresh.status !== 200) {
      throw new Error(`refresh ${fresh.status}`);
    }
    const stale = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: renew,
    });
    if (stale.status === 200) {
      throw new Error("old refresh survived rotation");
    }
  });

  await check("a wrong verifier is refused", async () => {
    const pk = await pkce();
    const code = await dance(sailor.head, pk.challenge);
    const grip = await form("/token", {
      grant_type: "authorization_code",
      code,
      redirect_uri: "http://127.0.0.1:9999/cb",
      client_id: "portal",
      code_verifier: `${pk.verifier}-tamper`,
    });
    if (grip.status === 200) {
      throw new Error("PKCE bypassed");
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

async function form(
  path: string,
  fields: Record<string, string>,
): Promise<Reply> {
  const body = new URLSearchParams(fields).toString();
  const res = await fetch(`${base}${path}`, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body,
  });
  return { status: res.status, body: await parse(res), headers: res.headers };
}

async function dance(
  head: Record<string, string>,
  challenge: string,
): Promise<string> {
  const query = new URLSearchParams({
    response_type: "code",
    client_id: "portal",
    redirect_uri: "http://127.0.0.1:9999/cb",
    scope: "openid profile",
    state: "voyage",
    code_challenge: challenge,
    code_challenge_method: "S256",
  });
  const res = await fetch(`${base}/authorize?${query}`, {
    headers: head,
    redirect: "manual",
  });
  await res.body?.cancel();
  const loc = res.headers.get("location") ?? "";
  const code = new URL(loc).searchParams.get("code");
  if (!code) {
    throw new Error(`no code in ${loc || res.status}`);
  }
  return code;
}

async function pkce(): Promise<{ verifier: string; challenge: string }> {
  const verifier = `${await digest("voyage")}${await digest("anchor")}`;
  const raw = new TextEncoder().encode(verifier);
  const sum = await crypto.subtle.digest("SHA-256", raw);
  const challenge = b64url(new Uint8Array(sum));
  return { verifier, challenge };
}

async function attest(token: string): Promise<Record<string, string>> {
  const set = await get("/.well-known/jwks.json");
  const jwk = (set.body as { keys: Record<string, unknown>[] }).keys[0];
  const key = await crypto.subtle.importKey(
    "jwk",
    { ...jwk, ext: true },
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["verify"],
  );
  const [head, load, seal] = token.split(".");
  const data = new TextEncoder().encode(`${head}.${load}`);
  const sig = unb64url(seal);
  const ok = await crypto.subtle.verify(
    { name: "ECDSA", hash: "SHA-256" },
    key,
    sig as BufferSource,
    data as BufferSource,
  );
  if (!ok) {
    throw new Error("id token signature invalid");
  }
  return JSON.parse(new TextDecoder().decode(unb64url(load)));
}

function b64url(bytes: Uint8Array): string {
  let raw = "";
  for (const byte of bytes) {
    raw += String.fromCharCode(byte);
  }
  return btoa(raw).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function unb64url(text: string): Uint8Array {
  const pad = text.replace(/-/g, "+").replace(/_/g, "/");
  const raw = atob(pad + "=".repeat((4 - pad.length % 4) % 4));
  return Uint8Array.from(raw, (ch) => ch.charCodeAt(0));
}

type Face = { id: number; head: Record<string, string> };

async function enrol(
  crown: Record<string, string>,
  login: string,
): Promise<Face> {
  const code = `invite-${login}`;
  await post("/invite", { hash: await digest(code), note: login }, crown);
  const made = await post("/join", {
    code,
    login,
    name: login,
    pass: `pass-${login}`,
  });
  const id = (made.body as { id: number }).id;
  const back = await post("/login", { login, pass: `pass-${login}` });
  return { id, head: { cookie: cookie(back) } };
}

async function grant(
  crown: Record<string, string>,
  row: { who: number | string; verb: string; unit: string; scope: string },
): Promise<void> {
  const made = await post("/@grant", {
    who: String(row.who),
    verb: row.verb,
    unit: row.unit,
    scope: row.scope,
  }, crown);
  if (made.status !== 201) {
    throw new Error(`grant ${made.status}`);
  }
}

function app(slug: string): Record<string, unknown> {
  return {
    name: slug,
    slug,
    home: `https://${slug}.perish.top`,
    redirect: `https://${slug}.perish.top/cb`,
    secret: "",
    mode: "oidc",
  };
}

async function drop(
  path: string,
  head: Record<string, string>,
): Promise<number> {
  const res = await fetch(`${base}${path}`, { method: "DELETE", headers: head });
  await res.body?.cancel();
  return res.status;
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
