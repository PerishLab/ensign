# Design

Ensign owns the proof from possession to identity. Keel owns every resource
effect after that proof. A successful password, session, token, recovery, or
provider ceremony yields an Actor identity and continues through its ordinary
operator; service work continues through the service operator. Sudo appears
only in genesis and identity birth.

## Resources

- `Actor` is the identity root: unique login, display name, and user or service
  kind.
- `Pass` is the one-to-one password floor. Keel stores the row; Ensign owns the
  authentication ceremony.
- `Invite` is invitation-only enrollment, rooted at its issuer and ended on
  use. Expiry rides its lease.
- `Rescue` is a one-shot recovery possession rooted at an Actor.
- `Team` is the group unit. Crew grants expand from its live Actor membership.
- `App` is the common registry for OIDC and forward-auth clients.
- `Renew` is a refresh grant bound to Actor, client, and scope. Rotation ends
  the old live row before minting the next and never widens scope.
- Gate-owned Token and Session rows remain rooted at Actor.

All administration remains the ordinary six resource verbs. Ensign ceremonies
compose those primitives but do not create a local authorization ledger.

## Credentials

Pass, Rescue, Renew, Invite, Token, and Session are veiled credential units.
They are engine-governed and ceremony-written but never exposed through the
generic resource projection. No operator can author or read credential rows on
the wire.

Enrollment is one transaction: consume the Invite, create the Actor through
identity birth, create Pass, and establish the newborn self grant. Recovery
burns the matching Rescue possession, every remaining rescue code, and every
session before establishing a new password floor.

OIDC authorization requires `openid`, carries nonce into the identity token,
and accepts only HTTPS or loopback redirects. Access and identity tokens carry
distinct kinds. Forward-auth resolves the same operator and returns identity
headers only for a live, unbarred identity.

Token exchange (RFC 8693) trades a held ensign bearer of a live, unbarred
Actor for an access token whose audience is a `resource` App. It is the
non-interactive face for services and agents: it mints no identity token, no
refresh grant and no session, never accepts a password, and refuses an
audience that is not a registered `resource`. People still prove possession
through the password floor and OIDC code + PKCE.

## Browser

The API binary carries the same-origin browser surface from tracked HTML, CSS,
and JavaScript. `/login` uses the existing password/session ceremony and returns
only to a validated local destination; the default signed-in view stays put.
`/logout` requires an explicit action, revokes the existing session and expires
its HttpOnly cookie. Passwords are submitted in JSON request bodies and are not
retained by the browser code. The API and CLI recovery floor remains independent
of page availability. No separate web process or runtime asset acquisition is
introduced. These plain identity forms make no Crest/Design brand qualification.

## Possessions

The ES256 signing key is app-owned durable bytes created only by explicit
bootstrap and loaded read-only at runtime. Signing custody and sudo custody are
separate even when orchestration uses the same artifact grammar. Runtime must
never receive the sudo artifact.

A consumer key is bootstrap-minted possession for one declared audience: the
ceremony that holds sudo ensures a `resource` App and a consumer Actor of the
same slug, kind `svc`, born through identity birth and so holding only its own
row grant, none of ensign's, and keeps a bearer of that Actor at the
caller-selected destination. Runtime receives no consumer key either; the
consumer holds it and exchanges it.

Authorization codes and future ceremony challenges are in-process ephemera,
not Keel rows. Durable credentials use leased rows; protocol ephemera die with
the process. Bearer material comes from the operating system CSPRNG and is
never printed or persisted in authored configuration.

## Guidance

The installed `ensign` client owns managed product guidance and exact
code-addressed recovery. Its Skill keeps durable identity doctrine and routes
to authorities; it does not copy command grammar. The client help owns client
grammar, `ensign-api --help` remains the server grammar authority, and only
bootstrap or signing failures that require custody judgment name Cookbook
codes. Routine refusals stay self-contained.


## Public client

The public Cargo package `ensign-client` owns client protocol and retained login
custody. Its caller explicitly supplies the data home, endpoint and login input;
it reads no ambient environment and carries no Plumb, Locus, server or Keel edge.
The package and executable `ensign` keep the product's existing ENSIGN_ cascade
and schema-1 credential location. CLI configuration remains in the anchor crate.
Consumers use the public client instead of reading or exporting its login bearer.

Identity reconciliation exposes existing Actor.sub, independently of local row
IDs and display names. Discovery is an observed document, not a trust decision.
Callers qualify canonical issuer and subject, select a registered audience and
request a short-lived access token through RFC8693. Radicula owns shared JWT,
PKCE and exchange protocol. Ensign owns signing keys, live identity checks,
audience registration and login custody; responses establish no supplier role.

Supplier provider objects, roles, trust policies and permission attachments stay
with the resource product. No Ensign Actor kind, role table, cloud-account row
or Keel/gate semantics are introduced. Explicit SDK context lets Runseal's native
profile execution consume this pure client without a framework dependency,
process-global configuration or a new execution callback.
