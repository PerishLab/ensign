# ensign

Identity on keel: the ensign you sail under.

The first delivery product of the keel fleet — an IdP shaped by
subtracting authentik down to a sovereign personal core: OIDC
(code + PKCE) and forward-auth out, password + recovery codes +
sudo window at the floor, everything vendor-dependent deferred.

## Shape

- `crates/api` — the server; the keel caller.
- `crates/cli` — the client.
- `apps/web` + `packages/components` — the web face (pnpm workspace).
- `charts/ensign` — the helm delivery.

## Local development

`sidecar.toml` owns the local API/Web topology. It leases both ports, waits for
the API readiness record, passes that endpoint to Vite, and then starts Web:

```sh
sidecar start
sidecar status
sidecar stop
```

The CLI takes its API root explicitly through `ENSIGN_URL`, including the
stable `/api` suffix; it has no fixed local-port fallback.
