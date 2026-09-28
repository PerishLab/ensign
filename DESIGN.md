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

## Possessions

The ES256 signing key is app-owned durable bytes created only by explicit
bootstrap and loaded read-only at runtime. Signing custody and sudo custody are
separate even when orchestration uses the same artifact grammar. Runtime must
never receive the sudo artifact.

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
