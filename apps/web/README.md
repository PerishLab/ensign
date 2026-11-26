# Ensign Web

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

The sign-in surface covers sign-in, signed-in identity and explicit sign-out.
It retains native username/current-password autofill, safe local returns,
password clearing, bounded requests, retry and session-revocation semantics.

Ensign delivers the Web surface formally through dual-OCI images
(`ghcr.io/perishlab/ensign` for `ensign-api` and `ghcr.io/perishlab/ensign-web`
for `apps/web` via `deploy/web.Dockerfile`) and the default-enabled Web workload
in `charts/ensign` (`web.enabled: true`), with the Ingress splitting `/api` to
`ensign-api` and `/` to `ensign-web`. The legacy embedded browser surface
(`legacy.html`, `client.mjs`, `style.css`, and the API `site` module) is retired;
`protocol.mjs` remains as the shared client protocol module with unit coverage in
`tests/protocol.test.mjs`.

## Settings

`/settings` provides a lightweight Ensign app-level theme and tone quick-switch
backed by Folio (`/folio/v1/me/documents/preferences`) plus a direct entry to
the Folio Domain-Wide Settings Center, where shared defaults (`shared`),
per-app overrides, and extension preferences are managed. Folio-synced
presentation stays in memory only (cleared on logout or 401) and is never
cached in `localStorage`. Device color scheme is request context only.

`/client.json` is public deployment input containing `issuer`, `client`, an
optional `folio` URL (`http:` / `https:`) pointing to the Folio Domain-Wide
Settings Center, and an optional same-origin `observe` path (such as
`/api/observe`) for Locus browser telemetry. The checked-in empty object keeps
settings and observability inactive for unconfigured deployments; omitted or
invalid `folio` or `observe` values degrade gracefully as absent, while
unreadable `/client.json` surfaces an unknown/retry notice. In Kubernetes,
`charts/ensign` renders the `<release>-web-client` ConfigMap (`{}` when
unconfigured) and mounts it at `/web/dist/client.json` with a `checksum/client`
rollout annotation, and `@perishlab/design/vite`'s static server serves it with
`Cache-Control: no-store`. The web application loads `/client.json` through a
single shared cached-promise loader per page load across observability and
authorization flows. Ingress routes `/api` to Ensign and `/folio` (prefix
removed) to Folio. These values contain no keys, tokens, passwords or
preference facts.

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
Issue Artifacts.
