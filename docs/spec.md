# Stage law

Delivery contract for ensign, first product of the keel fleet. A stage
is done when its act is green under `runseal :guard`; no calendar.
Boundary: identity state, ceremonies, OIDC, forward-auth are in;
passkeys, email/SMS OTP, TOTP, risk control are DEFERRED-ABSORB
(convenience/defense layers, never floors, never cold-start
dependencies). SAML / LDAP / RADIUS / SCIM / flow engines / upstream
federation are permanently out.

## Delivery grammar

Each stage delivers laws (keel-side when the engine or a package must
grow), surface (models, grants, ceremonies, routes), and an **act** —
a scenario over the running binary. Engine gaps become keel issues,
keel PRs, `:ship` releases, then version bumps here.

## Stages

| Stage | Delivers | Act proves |
|-------|----------|------------|
| I0 | layout, four planes, guard, CI | guard green on main — DONE (#1) |
| I1 | identity floor: Actor + gate resolve-only wiring, invite join, password login (argon2), logout, recovery codes, sudo genesis | invite → join → login → whoami; wrong password 401; reused invite 409; rescue burns once |
| conform | store portability: `KEEL_PG=... runseal :act` (compose pg:5435) | every act green on sqlite AND postgres |
| I2 | directory: Team (crew), App registry, admin via @grant rows only | admin registers an App; member sees; stranger 404; team grant admits and revokes |
| I3 | OIDC provider: discovery, authorize (code+PKCE), token, userinfo, JWKS, Renew rotation | act dances full code+PKCE against the binary, verifies JWT via JWKS, refreshes, revokes |
| I4 | forward-auth: `/auth` probe for traefik forwardAuth | cookie 200 + identity headers; anonymous 401; barred operator refused |
| I5 | keys: mask lands in keel (P-1 live intersection), scoped api-keys, rotate/delete | key ≤ owner live; owner loses grant → key shrinks instantly; zero-scope key reads public only |
| I6 | web flagship (P-3): DESIGN.md constitution, atoms, login/portal/admin views, playwright visual loop + `:e2e` | e2e drives real login → portal; screenshots reviewed against the constitution |
| I7 | cli: auth, actor/team/app/invite admin | cli round-trips against the binary in an act |
| I8 | ship: api+web images, chart real (init Job → sudo Secret), `:ship` version train | compose runs the images; helm-layer validation is the user's manual gate |

## Must not

- Authentik API mimicry (paths, payload shapes).
- Business code beyond models, grants, seeds, ceremonies, and
  package wiring.
- Protocol ephemera as rows; key material as rows.
- A password industry: no reset email, strength meter, breach list,
  expiry or complexity rules — the floor is a primitive.
- A stage advanced past a red act.
- Engine or package workarounds living here instead of keel issues.
