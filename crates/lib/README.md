# Ensign client

Public package ensign-client owns the Ensign client protocol and retained login
custody. Client::open takes an explicit data home and reuses its schema-1
state/pat.json. Client::login takes explicit home, endpoint, login and password.
It reads no ambient environment and has no Plumb, Locus, server or Keel dependency.
The ensign CLI keeps ENSIGN_ configuration and the existing data location.

Client::identity returns the external Actor subject. Client::exchange uses the
Radicula RFC8693 contract to obtain a registered resource access token without
exporting the held login bearer. The secret-bearing result is exposed only to
the intended peer. Discovery is an observation, not an issuer trust assertion.
Callers qualify issuer, subject, audience, trusted keys and downstream authority;
a token response alone proves no cloud role or permission.

Requests refuse redirects and have a finite timeout. Login creates a private new
file before replacement and refuses an occupied pending file. Logout forgets the
captured data home only after confirmed revocation. The SDK emits no observation;
the CLI owns command observations. No supplier resources are modeled here.
