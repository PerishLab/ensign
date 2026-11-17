# Ensign client

The public client owns Ensign configuration and retained login custody. `Client::retained`
reuses the existing schema-1 credential under `ENSIGN_HOME/state/pat.json`.
The executable remains `ensign`; its package is `ensign-cli`. Configuration retains
its `ENSIGN_` prefix and platform data home.

`Client::identity` returns the external Actor subject alongside display attributes.
`Client::exchange` exchanges the privately held login bearer for a registered resource
access token using the Radicula RFC8693 contract. It never exports the login bearer.
The result is secret-bearing: callers explicitly expose it only to the intended peer.
`Client::discovery` exposes the endpoint’s public discovery document as an
observation, not a trusted issuer assertion. The caller selects and qualifies the
issuer, audience, subject and vendor trust,
verifies the returned JWT with trusted discovery keys, and owns downstream permissions.
An exchange response by itself proves no cloud role or permission.

Transport refuses redirects and has a finite request timeout. Login writes a private
new file before replacement; an occupied pending file refuses rather than overwriting
an uncertain previous write. Logout forgets custody only after confirmed revocation.
No cloud resource, cloud account or role is modeled here.


The reusable SDK has no observer dependency and emits no CLI telemetry. The CLI
continues to own its command observations; moving transport and custody into the
SDK removes their former private CLI function traces. Credential bytes, passwords,
subjects and endpoint values never enter observation payloads.
