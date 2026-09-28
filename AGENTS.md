# Agents

This repository is a keel **caller**: ensign, the identity product —
an authentik-shaped IdP built purely on keel. It validates the whole
delivery framework (version train, acts, release lane, ship) before
codehull rides the same rails. The laws of the engine live in
keel's `AGENTS.md` and source; this repo obeys them from the outside.

## Layout

- `crates/api` — the server; the keel caller (package `api`, bin
  `ensign-api`).
- `crates/cli` — the client (bin `ensign`); the anchor crate bearing the
  repository name.
- `charts/ensign` — the helm delivery; it declares version `0.0.0` and wharf
  stamps the release marker onto it. Its ConfigMap supplies `ensign.toml`.
- `Containerfile` — the server image. Its build context is only this file and
  an `ensign-api` binary beside it; it bakes in no configuration.
- `deploy` — the local compose surface and its `ensign.toml`. Build with
  `cargo build --release --locked --bin ensign-api`, copy
  `target/release/ensign-api` to the repository root (ignored), then
  `docker compose -f deploy/compose.yml up --build`. A `bootstrap` service
  runs first and keeps sudo in a volume the `api` service never mounts.
- `apps/web` — a placeholder `index.html` only. The web plane is deferred,
  not deleted; the seat is held so the delivery paradigm can return without
  re-litigating its place. The pnpm workspace files stay for the same reason.
- `DESIGN.md` — current identity-resource and ceremony doctrine.

Plumb's pre-commit guard proves every commit against its exact staged tree,
and `plumb guard .` shows what it runs. `helm lint` and the acts are run by
hand when a change touches the chart or the delivery.

One wrapper remains under `.runseal/wrappers`: `act.ts`, the acts harness.
The generic guard, init and land wrappers and the repository-owned Git hooks
are gone — Runseal 0.14 stopped hosting them and Plumb 0.18.6 admitted the
wrapperless shape.

## Product stance

- Ensign closes the identity-authentication domain: enrollment, credentials,
  sessions, recovery, API keys, OIDC, forward-auth, signing, and their
  API/Web/CLI ceremonies live here.
- `Operator` is the only steady-state keel seam. Ensign proves possession and
  supplies an operator; resource shape, grants, lifecycle, transactions,
  storage, and events remain keel vocabulary and behavior.
- Ensign ceremonies may compose keel resource primitives, but never retell
  grant construction or use ambient sudo for steady-state effects. Genesis
  and identity birth are the only possession-grounded resource exceptions.
- Auth cold start is vendor-free: **password primitive + recovery
  codes + sudo window** — the escape ladder (every layer's last exit
  depends only on lower-layer possession, never an external vendor).
- Passkeys / email OTP / SMS OTP / risk control are DEFERRED-ABSORB:
  convenience and defense layers, added gradually, never floors,
  never cold-start dependencies.
- OIDC (code + PKCE) and forward-auth are the two provider faces.
- No SAML / LDAP / RADIUS / SCIM / flow engine / expression policies /
  upstream federation — permanent cuts.
- Capability needs ride keel's gate primitives; engine gaps become keel issues,
  not a local ledger.
- The first administrator is an ordinary Actor plus an ordinary `@grant`.
  Ensign CLI exposes the operation, but it remains a sudo-authorized keel
  resource creation; no role field, bootstrap table, or special admin route.
- Cold start law has settled and lives in the Cold start section below. It is
  the authority, not a summary: the server image has separate bootstrap and serve
  operations; bootstrap composes Keel hotspots and caller-owned artifact
  destinations, while serve only verifies prerequisites and refuses a `sudo`
  artifact by name. Sudo and OIDC signing may share orchestration grammar but
  never custody, mounts, access, lifecycle, or domain ownership.

## Laws

- Ectropy owns syntax laws (single word, block/path <= 4, no comments);
  Plumb owns repository shape and the canonical `ectropy.toml`. Vocabulary is
  expressed by the executable source and checked by Ectropy.
- Dependency direction: ensign -> keel-gate -> keel, plus plumb for the
  config mechanism and the release identity only (never its vocabulary).
  Never a workspace sibling of keel; distribution follows keel's channel.
- Runtime env rides the cascade under the `api` package's `API_` prefix
  (`API_STORE_KIND` / `API_STORE_URL` / `API_STORE_PATH` / `API_FRESH` /
  `API_ISS`); the port override is keel's `KEEL_LISTEN_PORT`, translated
  from `SIDECAR_PORT` in `sidecar.toml`, never read in product code.
- Engine gaps become keel issues and registry releases, never local
  workarounds; architecture conflicts halt the thread and get raised.
- Never commit on `main`. Branch, let the guard prove the commit, then land it
  with `plumb land`. A landed seat is retired immediately.
- Releases run through Plumb and wharf, never by hand. `plumb.toml` declares
  the product, its authority, its skill, and two executables: the `ensign`
  CLI, installed on every target, and the `ensign-api` server, linux only and
  never installed. The server is placed in the image
  `ghcr.io/perishlab/ensign:<version>` built from the root `Containerfile`,
  and the chart is published as `oci://ghcr.io/perishlab/charts/ensign`, both
  under the same marker as the CLI. `plumb release open` cuts
  `release/<version>` from a guarded `main`, `plumb release stamp` marks it,
  and `plumb ship dispatch` hands the marker to wharf, which binds both
  executables and publishes the archives, the image and the chart. Both
  executables call `plumb::identity!("ENSIGN")` and print `<binary> <marker>`
  from `--version`; an unbound `ensign-api` answers `--version` and refuses
  every other operation. Each stable owes its changelog and its skill,
  written for it and consigned with
  `plumb depot consign --kind changelog|skill --dir`; `plumb release owed`
  lists what is still owed.

For local development, `sidecar start|status|stop` owns the API/Web topology.
The CLI receives the explicit API root, including `/api`, through `ENSIGN_URL`.

## Cold start

- A **bootstrap** is one startup ceremony with a beginning and an end, never a
  row, a role, a route, or a long-running mode.
- An **artifact** is one named secret with one caller-selected destination:
  `sudo` for Keel estate genesis, `signing` for the OIDC key.
- **Sudo** is genesis possession; it is never converted into an account or
  retained as a steady-state role.
- The **service Actor** is exactly one live `Actor` named `ensign`, kind `svc`,
  `barred=false`, holding ordinary Keel `@grant` rows.
- Custody and runtime are separate seats. The process that may mint or keep
  sudo is not the process that serves requests; when orchestration cannot hold
  that line, the binary holds it.
- Refusal over repair. A missing prerequisite is a named refusal and the
  listener never opens; duplicate or conflicting state is drift, never silently
  rewritten.
- The cold start is vendor-free. The password primitive, recovery codes, and
  the sudo window are the floor, and no layer's last exit depends on a vendor.
- Generate a credential once per secret and reuse it. `helm lint` and
  `helm template` are both silent about a second `randAlphaNum` call.
- Every retained artifact carries `helm.sh/resource-policy: keep`, so deleting
  one is a deliberate human act. The chart deploys no web plane.
- `crates/api/tests/{law,bootstrap}.rs` mechanize grant vocabulary, the
  `.sudo()` exception, `serve` refusing a `sudo` artifact, and the replay
  table. Run the suite instead of restating its verdicts.
