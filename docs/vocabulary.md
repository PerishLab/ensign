# Vocabulary

- `ensign` — this product; the identity root of the personal stack.
  Also the cli binary name.
- `api` — the server crate; the keel caller.
- `cli` — the client crate.
- `web` / `components` — the pnpm workspace: the vite app and the
  component library (sole style territory).
- `charts` — the helm delivery.
- `ladder` — the escape ladder: password → recovery codes → sudo
  window; every exit depends only on possession.
- `booth` — ceremony route state: core + the ensign svc operator.
- `join` — the enrollment ceremony: invite code → Actor + Pass,
  invite burned; a mini-genesis (sudo births, newborn owns itself).
- `card` — a live Invite row found by code hash.
- `birth` — sudo creates the Actor and its self-ownership grant.
- `mint` — replace own rescue codes (three, plaintext shown once).
- `revive` — login by burning one rescue code.
- `spare` — a live Rescue row matching a code.
- `shield` — the stored Pass hash for an actor.
- `lock` / `fits` — argon2 hash / verify.
- `sow` — seed the svc operator's ceremony grants.
- `hail` — idempotent svc actor lookup-or-create at boot.
- `rig` — boot ceremony: hail + gate rise + sow.
- `crumb` / `wild` / `digest` — cookie read / random hex / sha256
  (gate idioms).
- `act` — the staged scenario runner over the api binary (`:act`).
- `booth` also carries directory writes; `Team` is the crew group,
  `App` the OIDC/forward client registry.
- `enrol` / `grant` / `drop` — act helpers: register a user, seed a
  grant, delete a tie.
- `link` — the `url` atom, aliased to dodge local `url` bindings.
- `oidc` — the provider module: discovery, jwks, authorize, token,
  userinfo over the App registry.
- `forge` — mint the ES256 signing keypair (app-owned, boot-local).
- `Keys` — the signing/decoding key pair plus the public jwk.
- `Code` — an in-process authorization code (protocol ephemera, never
  a keel row); `dance` / `attest` / `pkce` are its act helpers.
- `grip` / `trade` / `renew` — issue tokens, exchange a code, rotate
  a refresh token.
- `warrant` — a live Renew row found by token hash.
- `Renew` — the refresh-token unit.
