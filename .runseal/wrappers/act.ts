import { cli, flags } from "@/lib/cli.ts";
import { bin } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const host = "127.0.0.1";
const port = 13500;
const origin = `http://${host}:${port}`;
const base = `${origin}/api`;

function usage(): void {
  io.print("Usage: runseal :act");
  io.print("");
  io.print("Ensign stage acts over the running binary.");
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
const run = await Deno.makeTempDir({ dir, prefix: "run-" });
const sudoPath = `${run}/sudo`;
const signingPath = `${run}/sign.pem`;
await Deno.writeTextFile(
  `${run}/ensign.toml`,
  `[listen]\nhost = "${host}"\nport = ${port}\nprefix = ""\n\n[store]\nkind = "file"\npath = "estate.db"\n\n[cache]\nkind = "memory"\n`,
);

io.print("==> build api");
await bin("cargo").run(["build", "--locked"], { cwd: root });

const pg = Deno.env.get("API_STORE_URL");
const bootstrapEnv: Record<string, string> = pg
  ? { API_STORE_KIND: "pg", API_STORE_URL: pg, API_FRESH: "true" }
  : {};
const serveEnv: Record<string, string> = pg ? { API_STORE_KIND: "pg", API_STORE_URL: pg } : {};
if (pg) {
  io.print("==> store: postgres");
}

io.print("==> bootstrap api");
await bin("cargo").run([
  "run",
  "--locked",
  "-p",
  "api",
  "--",
  "bootstrap",
  run,
  "--artifact",
  `sudo=file:${sudoPath}`,
  "--artifact",
  `signing=file:${signingPath}`,
], { cwd: root, env: bootstrapEnv });
const sudoToken = (await Deno.readTextFile(sudoPath)).trim();

io.print(`==> serve api on ${base}`);
const child = new Deno.Command("cargo", {
  args: [
    "run",
    "--locked",
    "-p",
    "api",
    "--",
    "serve",
    run,
    "--artifact",
    `signing=file:${signingPath}`,
  ],
  cwd: root,
  env: serveEnv,
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
  await check("the api lives only below its namespace", async () => {
    const bare = await fetch(`${origin}/health`);
    await bare.body?.cancel();
    if (bare.status !== 404) {
      throw new Error(`root health ${bare.status}`);
    }
  });
  const crown = { authorization: `sudo ${sudoToken}` };

  io.print("==> act 1: identity floor");
  await check("possession admits exactly once", async () => {
    const seat = await post("/invite", { note: "first" }, crown);
    const code = (seat.body as { code?: string }).code ?? "";
    if (seat.status !== 201 || !code) {
      throw new Error(`invite ${seat.status}`);
    }
    const open = await post("/invite", { note: "open" });
    if (open.status !== 401 && open.status !== 403) {
      throw new Error(`anon invite ${open.status}`);
    }
    const made = await post("/join", {
      code,
      login: "ada",
      name: "Ada",
      pass: "seaworthy",
    });
    if (made.status !== 201) {
      throw new Error(`join ${made.status}`);
    }
    const again = await post("/join", {
      code,
      login: "bob",
      name: "Bob",
      pass: "driftwood",
    });
    if (again.status !== 404) {
      throw new Error(`burned invite ${again.status}`);
    }
    const chit = await post("/invite", { note: "short" }, crown);
    const feeble = await post("/join", {
      code: (chit.body as { code: string }).code,
      login: "wisp",
      name: "Wisp",
      pass: "short",
    });
    if (feeble.status !== 400) {
      throw new Error(`weak password admitted ${feeble.status}`);
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

  await check("a rescue code burns and re-floors the account", async () => {
    const minted = await post("/mint", {}, { cookie: jar });
    const codes = (minted.body as { codes?: string[] }).codes ?? [];
    if (minted.status !== 201 || codes.length !== 3) {
      throw new Error(`mint ${minted.status}`);
    }
    const back = await post("/revive", {
      login: "ada",
      code: codes[0],
      pass: "refloor-anchor-8",
    });
    if (back.status !== 204) {
      throw new Error(`revive ${back.status}`);
    }
    const stale = await get("/whoami", { cookie: jar });
    if (stale.status !== 401) {
      throw new Error(`old session outlived recovery ${stale.status}`);
    }
    const sibling = await post("/revive", {
      login: "ada",
      code: codes[1],
      pass: "refloor-anchor-8",
    });
    if (sibling.status === 204) {
      throw new Error("sibling rescue code survived re-floor");
    }
    const spent = await post("/revive", {
      login: "ada",
      code: codes[0],
      pass: "refloor-anchor-8",
    });
    if (spent.status !== 401) {
      throw new Error(`spent code ${spent.status}`);
    }
    const worn = await post("/login", { login: "ada", pass: "seaworthy" });
    if (worn.status !== 401) {
      throw new Error(`old password outlived recovery ${worn.status}`);
    }
    const relog = await post("/login", {
      login: "ada",
      pass: "refloor-anchor-8",
    });
    jar = cookie(relog);
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

  await check("a taken login does not spend the invite", async () => {
    const pass = "rope-ladder-9";
    const card = await post("/invite", { note: "clash" }, crown);
    const code = (card.body as { code: string }).code;
    const held = await enrol(crown, "keeper");
    void held;
    const clash = await post("/join", {
      code,
      login: "keeper",
      name: "Second Keeper",
      pass,
    });
    if (clash.status !== 409) {
      throw new Error(`clash ${clash.status}`);
    }
    const fresh = await post("/join", {
      code,
      login: "keeper2",
      name: "Keeper Two",
      pass,
    });
    if (fresh.status !== 201) {
      throw new Error(`invite spent by the clash ${fresh.status}`);
    }
  });

  await check("changing the password takes the current one", async () => {
    const card = await post("/invite", { note: "gale" }, crown);
    const code = (card.body as { code: string }).code;
    await post("/join", {
      code,
      login: "galer",
      name: "Galer",
      pass: "first-anchor-8",
    });
    const held = await post("/login", { login: "galer", pass: "first-anchor-8" });
    const sid = (held.headers.get("set-cookie") ?? "").split(";")[0];
    const wrong = await fetch(`${base}/repass`, {
      method: "POST",
      headers: { cookie: sid, "content-type": "application/json" },
      body: JSON.stringify({ old: "not-the-password", pass: "second-anchor-8" }),
    });
    await wrong.body?.cancel();
    if (wrong.status !== 403) {
      throw new Error(`repass without the current password ${wrong.status}`);
    }
    const right = await fetch(`${base}/repass`, {
      method: "POST",
      headers: { cookie: sid, "content-type": "application/json" },
      body: JSON.stringify({ old: "first-anchor-8", pass: "second-anchor-8" }),
    });
    await right.body?.cancel();
    if (right.status !== 204) {
      throw new Error(`repass with the current password ${right.status}`);
    }
    const stale = await post("/login", { login: "galer", pass: "first-anchor-8" });
    if (stale.status !== 401) {
      throw new Error(`old password survived repass ${stale.status}`);
    }
    const fresh = await post("/login", { login: "galer", pass: "second-anchor-8" });
    if (fresh.status !== 201) {
      throw new Error(`new password refused ${fresh.status}`);
    }
  });

  await check("recovery sets the new password atomically", async () => {
    const card = await post("/invite", { note: "drift" }, crown);
    const code = (card.body as { code: string }).code;
    const joined = await post("/join", {
      code,
      login: "drifter",
      name: "Drifter",
      pass: "old-anchor-8",
    });
    if (joined.status !== 201) {
      throw new Error(`join ${joined.status}`);
    }
    const held = await post("/login", { login: "drifter", pass: "old-anchor-8" });
    const jar = held.headers.get("set-cookie") ?? "";
    const sid = jar.split(";")[0];
    const minted = await fetch(`${base}/mint`, {
      method: "POST",
      headers: { cookie: sid },
    });
    const codes = ((await minted.json()) as { codes: string[] }).codes;
    const back = await post("/revive", {
      login: "drifter",
      code: codes[0],
      pass: "new-harbor-9",
    });
    if (back.status !== 204) {
      throw new Error(`revive ${back.status}`);
    }
    const stale = await post("/login", { login: "drifter", pass: "old-anchor-8" });
    if (stale.status !== 401) {
      throw new Error(`old password survived ${stale.status}`);
    }
    const fresh = await post("/login", { login: "drifter", pass: "new-harbor-9" });
    if (fresh.status !== 201) {
      throw new Error(`new password refused ${fresh.status}`);
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
    const code = await dance(sailor.head, pk.challenge, "openid profile", "salt");
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
    if (grip.headers.get("cache-control") !== "no-store") {
      throw new Error("token response is cacheable");
    }
    const claims = await attest(bag.id_token);
    if (claims.iss !== base || claims.aud !== "portal") {
      throw new Error(`claims ${JSON.stringify(claims)}`);
    }
    const sub = claims.sub as string;
    if (!sub || sub === String(sailor.id) || claims.login !== "sailor") {
      throw new Error(`subject ${JSON.stringify(claims)}`);
    }
    const shown = await get(`/profile/${sub}`, {});
    const face = shown.body as { handle?: string; teams?: unknown };
    if (shown.status !== 200 || face.handle !== "sailor") {
      throw new Error(`profile ${shown.status} ${JSON.stringify(face)}`);
    }
    if (face.teams !== undefined) {
      throw new Error("the anonymous profile face leaked teams");
    }
    if (claims.nonce !== "salt" || claims.kind !== "id") {
      throw new Error(`nonce/kind ${JSON.stringify(claims)}`);
    }
    const seen = await get("/userinfo", {
      authorization: `Bearer ${bag.access_token}`,
    });
    if ((seen.body as { sub?: string; login?: string }).login !== "sailor") {
      throw new Error(`userinfo profile ${JSON.stringify(seen.body)}`);
    }
    renew = bag.refresh_token;
  });

  await check("authorize demands openid and honors scope", async () => {
    const pk = await pkce();
    const bare = await walk(sailor.head, pk.challenge, "profile");
    const loc = bare.headers.get("location") ?? "";
    if (loc.includes("code=")) {
      throw new Error("authorize issued a code without openid");
    }
    const code = await dance(sailor.head, pk.challenge, "openid");
    const grip = await form("/token", {
      grant_type: "authorization_code",
      code,
      redirect_uri: "http://127.0.0.1:9999/cb",
      client_id: "portal",
      code_verifier: pk.verifier,
    });
    const bag = grip.body as Record<string, string>;
    const seen = await get("/userinfo", {
      authorization: `Bearer ${bag.access_token}`,
    });
    const face = seen.body as { sub?: string; login?: string };
    if (!face.sub || face.sub === String(sailor.id) || face.login !== undefined) {
      throw new Error(`scope leaked profile ${JSON.stringify(face)}`);
    }
  });

  await check("anon authorize bounces to login with return", async () => {
    const pk = await pkce();
    const res = await walk({}, pk.challenge, "openid profile");
    if (res.status !== 303) {
      throw new Error(`anon authorize ${res.status}`);
    }
    const loc = res.headers.get("location") ?? "";
    if (!loc.startsWith("/login?return=%2Fapi%2Fauthorize")) {
      throw new Error(`bounce ${loc}`);
    }
  });

  await check("tokens and the probe carry team claims", async () => {
    const mate = await enrol(crown, "mate");
    const deck = await post("/Team", { name: "deck" }, crown);
    const hold = (deck.body as { id: number }).id;
    const tie = await post(`/Team/${hold}/members`, { right: mate.id }, crown);
    if (tie.status !== 201) {
      throw new Error(`crew tie ${tie.status}`);
    }
    const pk = await pkce();
    const code = await dance(mate.head, pk.challenge, "openid profile");
    const grip = await form("/token", {
      grant_type: "authorization_code",
      code,
      redirect_uri: "http://127.0.0.1:9999/cb",
      client_id: "portal",
      code_verifier: pk.verifier,
    });
    const bag = grip.body as Record<string, string>;
    const claims = await attest(bag.id_token) as unknown as {
      teams?: string[];
    };
    if (!claims.teams?.includes("deck")) {
      throw new Error(`id token teams ${JSON.stringify(claims.teams)}`);
    }
    const seen = await get("/userinfo", {
      authorization: `Bearer ${bag.access_token}`,
    });
    const face = seen.body as { teams?: string[] };
    if (!face.teams?.includes("deck")) {
      throw new Error(`userinfo teams ${JSON.stringify(face.teams)}`);
    }
    const pass = await get("/auth", mate.head);
    if (pass.headers.get("x-ensign-teams") !== "deck") {
      throw new Error(`probe teams ${pass.headers.get("x-ensign-teams")}`);
    }
  });

  await check("refresh binds to its client and rotates", async () => {
    const loose = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: renew,
    });
    if (loose.status === 200) {
      throw new Error("refresh without client_id accepted");
    }
    const wrong = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: renew,
      client_id: "intruder",
    });
    if (wrong.status === 200) {
      throw new Error("refresh accepted a mismatched client");
    }
    const fresh = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: renew,
      client_id: "portal",
    });
    if (fresh.status !== 200) {
      throw new Error(`refresh ${fresh.status}`);
    }
    if ((fresh.body as { scope?: string }).scope !== "openid profile") {
      throw new Error("refresh widened the scope");
    }
    const stale = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: renew,
      client_id: "portal",
    });
    if (stale.status === 200) {
      throw new Error("old refresh survived rotation");
    }
  });

  await check("recovery revokes outstanding refresh grants", async () => {
    const castaway = await enrol(crown, "castaway");
    const pk = await pkce();
    const code = await dance(castaway.head, pk.challenge, "openid profile");
    const grip = await form("/token", {
      grant_type: "authorization_code",
      code,
      redirect_uri: "http://127.0.0.1:9999/cb",
      client_id: "portal",
      code_verifier: pk.verifier,
    });
    const token = (grip.body as { refresh_token: string }).refresh_token;
    const minted = await fetch(`${base}/mint`, {
      method: "POST",
      headers: castaway.head,
    });
    const codes = ((await minted.json()) as { codes: string[] }).codes;
    const back = await post("/revive", {
      login: "castaway",
      code: codes[0],
      pass: "landfall-anchor-8",
    });
    if (back.status !== 204) {
      throw new Error(`revive ${back.status}`);
    }
    const dead = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: token,
      client_id: "portal",
    });
    if (dead.status === 200) {
      throw new Error("refresh grant outlived recovery");
    }
  });

  await check("a barred owner cannot refresh", async () => {
    const rope = await enrol(crown, "roper");
    const pk = await pkce();
    const code = await dance(rope.head, pk.challenge);
    const grip = await form("/token", {
      grant_type: "authorization_code",
      code,
      redirect_uri: "http://127.0.0.1:9999/cb",
      client_id: "portal",
      code_verifier: pk.verifier,
    });
    const held = (grip.body as { refresh_token: string }).refresh_token;
    const patch = await fetch(`${base}/Actor/${rope.id}`, {
      method: "PATCH",
      headers: { "content-type": "application/json", ...crown },
      body: JSON.stringify({ barred: "true" }),
    });
    await patch.body?.cancel();
    const shut = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: held,
      client_id: "portal",
    });
    if (shut.status === 200) {
      throw new Error("barred owner still refreshed");
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

  io.print("==> act 4: forward-auth");
  const rider = await enrol(crown, "rider");
  await check("the probe stamps the identity", async () => {
    const pass = await get("/auth", rider.head);
    if (pass.status !== 200) {
      throw new Error(`probe ${pass.status}`);
    }
    if (pass.headers.get("x-ensign-login") !== "rider") {
      throw new Error(`no identity header`);
    }
    const anon = await get("/auth");
    if (anon.status !== 401) {
      throw new Error(`anon probe ${anon.status}`);
    }
  });

  await check("a barred operator is turned away", async () => {
    const patch = await fetch(`${base}/Actor/${rider.id}`, {
      method: "PATCH",
      headers: { "content-type": "application/json", ...crown },
      body: JSON.stringify({ barred: "true" }),
    });
    await patch.body?.cancel();
    if (patch.status !== 200 && patch.status !== 204) {
      throw new Error(`bar ${patch.status}`);
    }
    const shut = await get("/auth", rider.head);
    if (shut.status !== 401) {
      throw new Error(`barred probe ${shut.status}`);
    }
    const door = await post("/login", { login: "rider", pass: "pass-rider" });
    if (door.status !== 403) {
      throw new Error(`barred login ${door.status}`);
    }
  });

  io.print("==> act 5: credential units are veiled");
  const mallory = await enrol(crown, "mallory");
  await check("no operator can author a credential on the wire", async () => {
    const seal = await digest("forged");
    const forged = await post("/Renew", {
      hash: seal,
      slug: "portal",
      actor: String(mallory.id),
    }, mallory.head);
    if (forged.status !== 404) {
      throw new Error(`Renew authorable on the wire ${forged.status}`);
    }
    const redeem = await form("/token", {
      grant_type: "refresh_token",
      refresh_token: "forged",
    });
    if (redeem.status === 200) {
      throw new Error("forged refresh redeemed");
    }
    for (const unit of ["Pass", "Rescue", "Invite", "Session"]) {
      const shut = await post(`/${unit}`, { hash: "x" }, mallory.head);
      if (shut.status !== 404) {
        throw new Error(`${unit} authorable on the wire ${shut.status}`);
      }
      const read = await get(`/${unit}`, mallory.head);
      if (read.status !== 404) {
        throw new Error(`${unit} readable on the wire ${read.status}`);
      }
    }
    const query = await post("/query", { q: "from Pass" }, mallory.head);
    if (query.status !== 404) {
      throw new Error(`veiled unit queryable ${query.status}`);
    }
  });

  io.print("==> act 6: cli");
  const exe = `${root}/target/debug/ensign`;
  const nest = `${run}/cli`;
  const belt = { ENSIGN_URL: base, ENSIGN_HOME: nest };
  const pilot = await enrol(crown, "pilot");
  await grant(crown, { who: pilot.id, verb: "put", unit: "Invite", scope: "all" });
  await grant(crown, { who: pilot.id, verb: "put", unit: "Team", scope: "all" });
  await grant(crown, { who: pilot.id, verb: "put", unit: "App", scope: "all" });

  await check("the cli signs in and rounds a full admin trip", async () => {
    const login = await drive(exe, ["login", "pilot"], belt, "pass-pilot");
    if (login.code !== 0) {
      throw new Error(`cli login ${login.code}: ${login.err.trim()}`);
    }
    const who = await drive(exe, ["whoami"], belt);
    if (who.code !== 0 || !who.out.includes("pilot")) {
      throw new Error(`cli whoami ${who.code}: ${who.out.trim()}`);
    }
    const inv = await drive(exe, ["invite", "cli-made"], belt);
    if (inv.code !== 0 || inv.out.trim().length < 32) {
      throw new Error(`cli invite ${inv.code}: ${inv.out.trim()}`);
    }
    const team = await drive(exe, ["team", "flight"], belt);
    if (team.code !== 0) {
      throw new Error(`cli team ${team.err.trim()}`);
    }
    const app = await drive(
      exe,
      ["app", "Deck", "deck", "https://deck.lab", "https://deck.lab/cb"],
      belt,
    );
    if (app.code !== 0) {
      throw new Error(`cli app ${app.err.trim()}`);
    }
    const apps = await drive(exe, ["apps"], belt);
    if (!apps.out.includes("deck")) {
      throw new Error(`cli apps missing deck: ${apps.out.trim()}`);
    }
    const out = await drive(exe, ["logout"], belt);
    if (out.code !== 0) {
      throw new Error(`cli logout ${out.err.trim()}`);
    }
    const gone = await drive(exe, ["whoami"], belt);
    if (gone.code === 0) {
      throw new Error("cli whoami survived logout");
    }
  });

  await check("the cli redeems an invite and burns a rescue code", async () => {
    const seat = await post("/invite", { note: "cli-join" }, crown);
    const card = (seat.body as { code?: string }).code ?? "";
    if (seat.status !== 201 || !card) {
      throw new Error(`invite ${seat.status}`);
    }
    const nest = { ENSIGN_URL: base, ENSIGN_HOME: `${run}/cli-join` };
    const made = await drive(exe, ["join", card, "cadet", "Cadet"], nest, "pass-cadet");
    if (made.code !== 0 || !made.out.includes("cadet")) {
      throw new Error(`cli join ${made.code}: ${made.err.trim()}`);
    }
    const first = await drive(exe, ["login", "cadet"], nest, "pass-cadet");
    if (first.code !== 0) {
      throw new Error(`cli login after join ${first.err.trim()}`);
    }
    const kept = await Deno.readTextFile(`${run}/cli-join/state/pat.json`);
    const held = JSON.parse(kept) as { schema?: number; token?: string };
    if (held.schema !== 1) {
      throw new Error(`credential schema ${held.schema}`);
    }
    const pat = held.token ?? "";
    const codes = await post("/mint", {}, { authorization: `token ${pat}` });
    const spare = ((codes.body as { codes?: string[] }).codes ?? [])[0] ?? "";
    if (codes.status !== 201 || !spare) {
      throw new Error(`mint ${codes.status}`);
    }
    const back = await drive(exe, ["recover", "cadet", spare], nest, "pass-cadet-two");
    if (back.code !== 0) {
      throw new Error(`cli recover ${back.code}: ${back.err.trim()}`);
    }
    const stale = await drive(exe, ["login", "cadet"], nest, "pass-cadet");
    if (stale.code === 0) {
      throw new Error("the old password still signs in after recovery");
    }
    const fresh = await drive(exe, ["login", "cadet"], nest, "pass-cadet-two");
    if (fresh.code !== 0) {
      throw new Error(`cli login after recovery ${fresh.err.trim()}`);
    }
    const twice = await drive(exe, ["recover", "cadet", spare], nest, "pass-cadet-three");
    if (twice.code === 0) {
      throw new Error("a burned rescue code recovered a second time");
    }
  });

  await check("the cli crowns a site admin with sudo from stdin", async () => {
    const seat = await post("/invite", { note: "cli-crown" }, crown);
    const card = (seat.body as { code?: string }).code ?? "";
    const nest = { ENSIGN_URL: base, ENSIGN_HOME: `${run}/cli-crown` };
    const made = await drive(exe, ["join", card, "regent", "Regent"], nest, "pass-regent");
    if (made.code !== 0) {
      throw new Error(`cli join ${made.err.trim()}`);
    }
    const first = await drive(exe, ["login", "regent"], nest, "pass-regent");
    if (first.code !== 0) {
      throw new Error(`cli login ${first.err.trim()}`);
    }
    const before = await drive(exe, ["team", "regency"], nest);
    if (before.code === 0) {
      throw new Error("an ordinary actor founded a team before being crowned");
    }
    const wrong = await drive(exe, ["crown", "regent"], nest, "deadbeef");
    if (wrong.code === 0) {
      throw new Error("a bad sudo token crowned an actor");
    }
    const rite = await drive(exe, ["crown", "regent"], nest, sudoToken);
    if (rite.code !== 0) {
      throw new Error(`cli crown ${rite.code}: ${rite.err.trim()}`);
    }
    const after = await drive(exe, ["team", "regency"], nest);
    if (after.code !== 0) {
      throw new Error(`crowned actor still refused ${after.err.trim()}`);
    }
  });

  await check("a stranger cli is refused, not crashed", async () => {
    const hand = await enrol(crown, "stow");
    const login = await drive(exe, ["login", "stow"], belt, "pass-stow");
    if (login.code !== 0) {
      throw new Error(`cli login ${login.err.trim()}`);
    }
    const app = await drive(
      exe,
      ["app", "Sneak", "sneak", "https://x.lab", "https://x.lab/cb"],
      belt,
    );
    if (app.code === 0 || !app.err.includes("refused")) {
      throw new Error(`stranger app ${app.code}: ${app.err.trim()}`);
    }
    void hand;
    await drive(exe, ["logout"], belt);
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

async function walk(
  head: Record<string, string>,
  challenge: string,
  scope: string,
  nonce?: string,
): Promise<Reply> {
  const query = new URLSearchParams({
    response_type: "code",
    client_id: "portal",
    redirect_uri: "http://127.0.0.1:9999/cb",
    scope,
    state: "voyage & drift",
    code_challenge: challenge,
    code_challenge_method: "S256",
  });
  if (nonce) {
    query.set("nonce", nonce);
  }
  const res = await fetch(`${base}/authorize?${query}`, {
    headers: head,
    redirect: "manual",
  });
  await res.body?.cancel();
  return { status: res.status, body: null, headers: res.headers };
}

async function dance(
  head: Record<string, string>,
  challenge: string,
  scope = "openid profile",
  nonce?: string,
): Promise<string> {
  const res = await walk(head, challenge, scope, nonce);
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
  const seat = await post("/invite", { note: login }, crown);
  const code = (seat.body as { code: string }).code;
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

async function drive(
  exe: string,
  args: string[],
  belt: Record<string, string>,
  feed?: string,
): Promise<{ code: number; out: string; err: string }> {
  const cmd = new Deno.Command(exe, {
    args,
    env: belt,
    stdin: feed === undefined ? "null" : "piped",
    stdout: "piped",
    stderr: "piped",
  });
  const child = cmd.spawn();
  if (feed !== undefined) {
    const writer = child.stdin.getWriter();
    await writer.write(new TextEncoder().encode(feed));
    await writer.close();
  }
  const done = await child.output();
  return {
    code: done.code,
    out: new TextDecoder().decode(done.stdout),
    err: new TextDecoder().decode(done.stderr),
  };
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
