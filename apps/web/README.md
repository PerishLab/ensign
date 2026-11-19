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
