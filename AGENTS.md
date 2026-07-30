# Agents

This repository is a keel **caller**: ensign, the identity product —
an authentik-shaped IdP built purely on keel. It validates the whole
delivery framework (four planes, version train, acts/e2e, ship) before
codehull rides the same rails. The laws of the engine live in
`keel:docs/*`; this repo obeys them from the outside.

## Layout

A monorepo of four delivery planes, split by toolchain:

- `crates/api` — the server; the keel caller (bin `api`).
- `crates/cli` — the client (bin `ensign`).
- `apps/web` — the pnpm web application (node 24, vite, react,
  typescript, vitest, biome); every version is pinned in the
  `pnpm-workspace.yaml` catalog and dependencies reference `catalog:` only.
- `charts/ensign` — the helm delivery.

Territory: application components live under
`apps/web/src/lib/components`, remain style-free, and consume the Design
runtime for reusable visual behavior. `runseal :guard` spans all planes:
cargo fmt/clippy/test, biome/tsc/vitest, helm lint, Plumb, Ectropy, and acts.

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
- Capability needs ride keel's gate primitives (`keel:docs/run/capability.md`
  § Gate); engine gaps become keel issues, not a local ledger.
- The first administrator is an ordinary Actor plus an ordinary `@grant`.
  Ensign CLI exposes the operation, but it remains a sudo-authorized keel
  resource creation; no role field, bootstrap table, or special admin route.
- Cold start law is under active revision and is held outside the repository
  while it settles. The settled floor: the server image has separate
  bootstrap and serve operations; bootstrap composes Keel hotspots and
  caller-owned artifact destinations, while serve only verifies prerequisites.
  Sudo and OIDC signing may share orchestration grammar but never custody,
  mounts, access, lifecycle, or domain ownership.

## Laws

- Ectropy owns syntax laws (single word, block/path <= 4, no comments);
  Plumb owns repository shape and the canonical `ectropy.toml`. Vocabulary
  deltas remain documented in `docs/vocabulary.md`.
- Dependency direction: ensign -> keel-gate -> keel, plus plumb for the
  config mechanism only (never its vocabulary). Never a workspace sibling
  of keel; distribution follows keel's channel.
- Runtime env rides the cascade under the binary's own `API_` prefix
  (`API_STORE_KIND` / `API_STORE_URL` / `API_STORE_PATH` / `API_FRESH` /
  `API_ISS`); the port override is keel's `KEEL_LISTEN_PORT`, translated
  from `SIDECAR_PORT` in `sidecar.toml`, never read in product code.
- Engine gaps become keel issues and registry releases, never local
  workarounds; architecture conflicts halt the thread and get raised.
- Never commit on `main`; branch, then `runseal :guard` and
  `runseal :land`.
