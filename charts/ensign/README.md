# Ensign Helm lifecycle

Resolve the formal product release through Plumb and Wharf before selecting its
chart and images. The chart consumes one API instance, one Web instance (when
`web.enabled` is true), and a durable PostgreSQL store. Set a stable issuer
explicitly; a local validation issuer does not establish public TLS or browser
authentication. Public GHCR images need no registry credentials; set top-level
`imagePullSecrets` (e.g. `[{name: registry}]`) only when pulling from a private
registry or mirror, which wires credentials across API, bootstrap, web, and
bundled PostgreSQL pods.

## Install

Bootstrap runs as a separate, revision-named ordinary Job. Its identity can
create custody Secrets and read the retained sudo and signing artifacts. The
serving identity can get only its named signing Secret. PostgreSQL, bootstrap
and the API may start together; bootstrap has six retries and a ten-minute
deadline, and serve refuses missing prerequisites rather than initializing them.
Use Helm's wait and wait-for-jobs options with a fifteen-minute timeout to
observe both outcomes, including the API restart backoff after bootstrap.

Bootstrap keeps sudo and signing itself, outside Helm's owned resource set.
The chart preserves the database credential and PostgreSQL volume. Retain these
as a matched recovery set; uninstalling or recreating an application is not a
reason to replace its identity. API restarts reuse the completed ceremony;
explicit Helm operations create a new bounded replay Job.

## Upgrade

The API uses Recreate and accepts exactly one replica. Expect a brief service
interruption while the old API stops before the replacement starts. This is a
Deployment update guarantee, not fencing for forced deletion or a failed node.

For an existing installation made with Helm 4 server-side apply, use its
client-side update mode when changing the legacy default RollingUpdate strategy:

```sh
helm upgrade RELEASE CHART --namespace NAMESPACE --server-side=false \
  --wait --wait-for-jobs --timeout 15m --values VALUES
```

CHART and VALUES name the verified release chart and the instance's configuration.
Keep existing issuer and store configuration. Helm 3 already uses client-side
updates and does not take the server-side flag. Helm 4's server-side update can
retain the old default rollingUpdate field and receive Kubernetes' validation
refusal when switching to Recreate; client-side update removes that legacy field.
Do not use force replacement or delete the store/custody to make the upgrade pass.

A refusal can leave a completed bootstrap Job beside an unchanged old API.
Inspect the exact Helm revision and live resources before continuing; do not
claim the new runtime identity is active until its Deployment is ready and its
permissions are checked. Verify that serve cannot get sudo or write Secrets,
health and discovery recover, and the signing and database identities remain.

Complete or inspect the preceding Helm operation before starting another. Jobs
can replay, and a still-running older revision can overlap a new ceremony; the
binary retains its refusal and conflict handling. Each operation has its own
bootstrap Job, with finished Job evidence retained for up to one day. An older chart rollback also restores that chart's permissions and
update policy. Recovery and formal rollout acceptance remain separate operations.

## Web surface

Ensign ships as a dual-OCI release (`ghcr.io/perishlab/ensign` for `ensign-api`
and `ghcr.io/perishlab/ensign-web` for the independent Svelte Web surface). The
chart enables `web.enabled: true` by default, deploying a separate `*-web`
Deployment and Service serving the built `dist/.perish/server.mjs` and splitting
the Ingress so `/api` reaches the API service (JSON and OIDC endpoints) while
`/` reaches the web service (`/login`, `/logout`, `/settings`, `/callback`).

Use a dedicated issuer host so this root route does not take over another
application's paths. Browser session logout is distinct from CLI bearer logout;
the page revokes the session and clears its cookie. The embedded HTML/JS/CSS
pages have been retired from the API binary; if you disable `web.enabled`, the
API binary serves only `/api` and does not serve browser pages, so the operator
must provide the Web surface separately.
