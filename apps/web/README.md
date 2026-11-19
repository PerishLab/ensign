# Ensign Web candidate

The independent Svelte application consumes Design through its public component
and Vite entries. Ensign owns identity requests, state, copy and navigation;
Design owns the visual language and component behavior.

`src/views/*.svelte` are the route files: Design's Vite entry scans them into
`virtual:perish/views`, and `src/main.ts` mounts Design's `Views` over that
catalog, so this application declares no router of its own. Shared markup lives
in `src/lib/components`.

`pnpm --filter @perishlab/ensign-web build` produces `dist/` and its independent
`dist/.perish/server.mjs`. The product's Sidecar `web` target runs development;
`web-built` serves the built artifact. Both use allocated ports. Development
accepts an explicit `API_URL` through Design's normal proxy configuration;
the built server expects the ingress to route `/api` to the identity API.

The first surface covers sign-in, signed-in identity and explicit sign-out.
It retains native username/current-password autofill, safe local returns,
password clearing, bounded requests, retry and session-revocation semantics.

This is the candidate stage of Ensign#114. During preparation, `legacy.html`
keeps the API-carried surface usable. Its client and the new application share
`protocol.mjs`; the API also serves that module. The final chart/routing cutover
will retire this embedded surface. Candidate package overrides belong to the
Issue Artifact, not to the formal dependency declaration or release claim.

## Settings candidate

Ensign#115 uses the public Design Preference in compact and aggregate views.
The browser reads Folio app/shared snapshots and sends revisioned intents;
application presentation always comes from the app snapshot, including while
editing shared declarations. Device color scheme is request context only.

`/client.json` is public deployment input containing `issuer` and `client`.
The checked-in empty object keeps settings inactive for unconfigured deployments.
Development and production supply it through the deployment routing. K3s routes
`/client.json` to the public configuration seat and `/folio` to Folio, removing
that prefix and the Ensign cookie while preserving the resource bearer.
The built static server expects deployment to provide `client.json` and ingress
to route `/api` to Ensign and `/folio` (prefix removed) to Folio. These values
contain no keys, tokens, passwords or preference facts.

The issuer remains canonical even when identity transport is same-origin.
Register the exact `/callback` redirect for the configured OIDC client and the
`folio` resource audience. The browser binds discovery, origin, client, return,
state and S256 verifier; it consumes the transaction once and removes callback
parameters before exchanging the code. Only the access token stays in memory.
Session storage holds the pending authorization transaction, never preferences
or access/refresh tokens. Reload follows the existing session through PKCE again;
logout clears browser custody after confirmed session revocation.

The protocol tests use Node24's native experimental Web Storage implementation.
Real authentication, persistence, failure and browser evidence belong to the
Issue Artifacts. The final multi-image/chart cutover remains Ensign#114 work;
this source candidate does not claim an already released web topology.
