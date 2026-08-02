# Agents

This repository is a keel **caller**: ensign, the identity product —
an authentik-shaped IdP built purely on keel. It validates the whole
delivery framework (version train, acts, release lane, ship) before
codehull rides the same rails. The laws of the engine live in
`keel:docs/*`; this repo obeys them from the outside.

## Layout

- `crates/api` — the server; the keel caller (bin `api`, staged and shipped
  as `ensign-api`).
- `crates/cli` — the client (bin `ensign`); the anchor crate bearing the
  repository name.
- `charts/ensign` — the helm delivery.
- `deploy` — the runtime image and the compose surface.
- `skills/ensign` — the bootstrap law, authoritative rather than a summary.
- `apps/web` — a placeholder `index.html` only. The web plane is deferred,
  not deleted; the seat is held so the delivery paradigm can return without
  re-litigating its place. The pnpm workspace files stay for the same reason.

Guard runs from `.forgejo/workflows/guard.yml` and calls its checkers
directly: Plumb, Ectropy, cargo fmt/clippy/test, the release profile, deno
fmt/check, helm lint where helm exists, and the acts.

Two wrappers remain under `.runseal/wrappers`, each with a role Plumb
recognises: `act.ts` is the acts harness, and `ship.ts` builds and pushes the
api image and the chart. The generic guard, init and land wrappers and the
repository-owned Git hooks are gone — Runseal 0.14 stopped hosting them and
Plumb 0.18.6 admitted the wrapperless shape.

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
- Cold start law has settled and lives in `skills/ensign/SKILL.md`. It is the
  authority, not a summary: the server image has separate bootstrap and serve
  operations; bootstrap composes Keel hotspots and caller-owned artifact
  destinations, while serve only verifies prerequisites and refuses a `sudo`
  artifact by name. Sudo and OIDC signing may share orchestration grammar but
  never custody, mounts, access, lifecycle, or domain ownership.

## Laws

- Ectropy owns syntax laws (single word, block/path <= 4, no comments);
  Plumb owns repository shape and the canonical `ectropy.toml`. Vocabulary
  deltas remain documented in `docs/vocabulary.md`.
- Dependency direction: ensign -> keel-gate -> keel, plus plumb for the
  config mechanism and the build version stamp only (never its vocabulary).
  Never a workspace sibling of keel; distribution follows keel's channel.
- Runtime env rides the cascade under the binary's own `API_` prefix
  (`API_STORE_KIND` / `API_STORE_URL` / `API_STORE_PATH` / `API_FRESH` /
  `API_ISS`); the port override is keel's `KEEL_LISTEN_PORT`, translated
  from `SIDECAR_PORT` in `sidecar.toml`, never read in product code.
- Engine gaps become keel issues and registry releases, never local
  workarounds; architecture conflicts halt the thread and get raised.
- Never commit on `main`. Branch, let the guard lane prove the commit, then
  land through a pull request. A landed seat is retired immediately: a
  rebase-landed worktree stops being provably landed once `main` moves again.
- Releases run through Plumb, never by hand: `plumb release dispatch` for an
  exact candidate, then `plumb stable prepare / pick / freeze`, promotion, and
  `plumb stable packport`. `plumb.toml` declares the product; the two callers
  in `.forgejo/workflows` stay thin and must keep declaring `guard_contexts`,
  which is what makes a release read the guard evidence already recorded
  against the commit instead of recomputing it against a moving registry.
