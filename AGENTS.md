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
- `apps/web` + `packages/components` — the pnpm workspace (node 24,
  vite, react, typescript, vitest, biome); every version pinned in the
  `pnpm-workspace.yaml` catalog, packages reference `catalog:` only.
- `charts/ensign` — the helm delivery.

Territory: only `packages/components` owns style declarations; apps
consume classNames and declare nothing. `runseal :guard` spans all
planes: cargo fmt/clippy/test, biome/tsc/vitest, helm lint, negentropy
(acts join when the api grows its first unit).

## Product stance

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

## Laws

- Negentropy laws apply (single word, block/path <= 4, no comments);
  vocabulary deltas in `docs/vocabulary.md`.
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
