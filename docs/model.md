# Model

The slice, over keel primitives. Engine and package gaps found here
become keel issues and registry releases, never local workarounds.

## Units

- `Actor` — the identity unit (`keel.toml`). `login` string unique,
  `name` string, `kind` string (user | svc).
- `Pass` — the password floor (escape ladder rung 1). `hash` string
  (argon2id, app-verified), one2one root at Actor. keel stores the
  column; the ceremony lives in api middleware — keel never
  authenticates.
- `Invite` — enrollment is invitation only. `hash` string unique,
  `note` string, many2one root at the issuing Actor. Ended on use;
  expiry rides lease.
- `Rescue` — recovery codes (escape ladder rung 2). `hash` string,
  many2one root at Actor. One-shot: ended on use.
- `Team` — the group unit. `name` string unique, `members` many2many
  Actor with the `crew` marker (group grants expand live).
- `App` — the application registry, one unit for both provider faces.
  `name` string, `slug` string unique, `home` url, `redirect` url,
  `secret` string (hash, empty for public PKCE clients), `mode`
  string (oidc | forward).
- `Renew` — OIDC refresh grant. `hash` string unique, `slug` string
  (the client), many2one root at Actor. Rotated on use; expiry rides
  lease. The App tie is carried by slug, not an edge (refresh reads
  it flat).
- `Token` / `Session` — gate package units (bearer + cookie),
  rooted at Actor.
- `@grant` — engine unit; all administration is six verbs on rows.

## Veil (credential units off the wire)

`Pass`, `Rescue`, `Renew`, `Invite`, and gate's `Token`/`Session` are
`#[resource(veil)]` (keel C-18): engine-governed, Face-written by
ceremonies, but off the generic HTTP projection. No operator can author
or read a credential row on the wire — closes the self-mint bypass.
Issuance is a ceremony: `/invite` mints a server-CSPRNG code (never
client-chosen); `/join` consumes it; `/revoke` (gate) ends a token.

## Keys (signing material)

The ES256 signing key is app-owned bytes, born once at genesis and
persisted (`<root>/.local/sign.pem`), loaded thereafter — same
possession model as the sudo token, and it survives restart (no more
ephemeral-per-boot keys). `kid` derives from the public key. Bearer
secrets everywhere draw 256 bits from the OS CSPRNG (`getrandom`),
never a hash-table hasher.

## Not rows (protocol ephemera law)

Authorization codes (60 s) and any future ceremony challenges are
in-process state, never keel rows. Durable credentials are rows with
lease-ridden expiry; ephemera die with the process.

## Key material

JWT signing keys are app-owned bytes (`.local/keys/`, k8s Secret in
the chart) — never keel rows, mirroring the blob-plane split: keel
holds authority, not secrets it need not read.

## Gaps surfaced (dream-code, 2026-07-17)

- keel-gate: needs a **resolve-only public surface** — today `wall()`
  welds the stock doors (/register token-flow) to the pass layer;
  ensign brings its own ceremonies (invite + password) and wants the
  resolution middleware alone. keel issue → keel-gate release.
- mask (P-1, keel seat): scoped api-keys enter at stage I5; v1 gate
  Tokens are unscoped full-authority bearers.
- No other engine gaps: unique/lease/crew/@grant/bar cover the slice.
