# Hot paths

## Bring a deployment up

Bootstrap runs in one fixed order and is resumable at every commit edge:

1. Resolve independent artifact destinations.
2. Open the store and the Keel bootstrap surface.
3. Load sudo, or mint and durably keep it.
4. Transactionally seal or exactly replay the estate.
5. Find or create the canonical service Actor.
6. Explicitly sow the Keel-gate and Ensign service grants.
7. Load and validate signing, or generate and durably keep it.
8. Verify every runtime prerequisite, then exit success.

Only explicit bootstrap may idempotently restore the declared grant baseline.

## Serve

`api serve` checks an ordinary Keel bind, the service Actor, every declared
grant, a valid signing artifact, and an explicit stable issuer, then listens. It
creates no Actor, grant, gate seed, key, directory, or artifact, refuses a `sudo`
artifact by name, and never falls through to bootstrap.

## Create the first administrator

Bootstrap creates no human, Invite, admin role, or wildcard grant. Once the API
is ready, open the first Invite through sudo custody, join as an ordinary Actor,
then create one site-admin grant. Sudo returns to dormant custody for recovery.

## Change the repository

Read `AGENTS.md`, then `docs/model.md` and `docs/vocabulary.md` as the change
requires. Run `plumb doctor .` and `ectropy .` around a shape change, and the
complete guard in `.forgejo/workflows/guard.yml` before landing.
