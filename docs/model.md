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
- `Renew` — OIDC refresh grant. `hash` unique, `slug` (the bound
  client), `scope` (the granted scope, never widened on refresh),
  many2one root at Actor. Refresh binds to `slug`, checks the owner is
  not barred and the client still exists, and rotates by ending the old
  row (a live-row CAS: a replayed or raced token ends-dead and is
  refused) before minting the next. Expiry rides lease. Residual
  (documented, single-instance defer): full family reuse-revocation —
  a token replayed after rotation is refused, but the rotated-ahead
  descendant is not proactively revoked.
- `Token` / `Session` — gate package units (bearer + cookie),
  rooted at Actor.
- `@grant` — engine unit; all administration is six verbs on rows.

## Operator seam

Ensign owns the proof from possession to identity. Keel owns every resource
effect after that proof. A successful password, session, token, recovery, or
provider ceremony yields an Actor id and continues through `Core::of(actor)`;
service work continues through the service operator. Sudo is used only through
keel's genesis/identity-birth primitives.

Invitation join is one transaction: consume the Invite, invoke gate's identity
birth primitive, and create Pass. Gate owns creation of the newborn self grant;
Ensign owns the invitation and password policy. Recovery proves the Actor with
a Rescue possession, then re-floors that Actor through its ordinary face.

## Veil (credential units off the wire)

`Pass`, `Rescue`, `Renew`, `Invite`, and gate's `Token`/`Session` are
`#[resource(veil)]` (keel C-18): engine-governed, Face-written by
ceremonies, but off the generic HTTP projection. No operator can author
or read a credential row on the wire — closes the self-mint bypass.
Issuance is a ceremony: `/api/invite` mints a server-CSPRNG code
(never client-chosen); `/api/join` consumes it; `/api/revoke` (gate)
ends a token.

## Token & ceremony hygiene

- Issuer is configurable (`API_ISS`), includes the stable `/api` root,
  and is not the bind address; session cookies carry `SameSite=Lax` and
  `Secure` behind https; `/api/token` and credential responses set
  `Cache-Control: no-store`.
- `/api/authorize` requires the `openid` scope, accepts `nonce` (echoed
  into the id token), builds its redirect with percent-encoding, and
  admits only https or loopback redirect targets. id and access tokens
  carry a `kind` claim; `/api/userinfo` accepts only `kind=access` and
  returns profile claims only when the token's scope grants them.
- `Pass` is one2one on Actor (one live credential); join consumes the
  invite first (a live-row CAS) and rejects passwords under 8 chars;
  recovery re-floors — burning every remaining rescue code and ending
  every session before minting the new one.
- Interpolated query values are scrubbed (backslash/quote) — belt over
  keel's and-only grammar, which is not injectable today.

## Keys (signing material)

The ES256 signing key is app-owned bytes, provisioned by explicit bootstrap
and loaded read-only by ordinary runtime. It and sudo are both durable
possessions but belong to different domains and custody contracts: runtime
needs signing and must not receive sudo. `kid` derives from the public key.
Bearer secrets draw 256 bits from OS CSPRNG (`getrandom`).

## Not rows (protocol ephemera law)

Authorization codes (60 s) and any future ceremony challenges are
in-process state, never keel rows. Durable credentials are rows with
lease-ridden expiry; ephemera die with the process.

## Key material

JWT signing keys are app-owned bytes (local artifact or k8s Secret) — never
Keel rows. Provision, replay, and runtime refusal follow the cold start law.

## Gaps surfaced (dream-code, 2026-07-17)

- keel-gate: needs a **resolve-only public surface** — today `wall()`
  welds the stock doors (/register token-flow) to the pass layer;
  ensign brings its own ceremonies (invite + password) and wants the
  resolution middleware alone. keel issue → keel-gate release.
- mask (P-1, keel seat): scoped api-keys enter at stage I5; v1 gate
  Tokens are unscoped full-authority bearers.
- No other engine gaps: unique/lease/crew/@grant/bar cover the slice.
