# Ensign v0.2.0

Ensign's cold start closes. The server image carries two separate process
commands: `bootstrap` composes Keel's hotspots and the caller's artifact
destinations, while `serve` only verifies prerequisites and refuses to open a
listener when one is missing. Sudo and OIDC signing share an orchestration
grammar but never custody, mounts, access, lifecycle, or domain ownership.

The Helm delivery now brings that ceremony up on a real cluster. Bootstrap runs
as an initContainer the API Pod owns rather than as a hook-driven Job, because
`helm install --wait` waits for readiness before running post-install hooks and
the Job was therefore never created. Sudo, signing, and store secrets and the
store volume all carry `helm.sh/resource-policy: keep`, so an uninstall leaves
custody intact and a reinstall claims the original estate instead of minting a
new one. Store credentials are generated once and reused, and the chart no
longer deploys a web plane.

Identity itself was split. What a person *is*, how they *sign in*, and how they
are *seen* are now three separate concerns rather than one row, and an access
token carries the resource it was actually asked for. Ensign also took its
configuration surface back from Keel and moved to Sealkit 0.3.

This is the first version to ship through the shared release lane. The
repository declares its product, authority, binaries, targets, and skill in
`plumb.toml`, and two thin callers hand the sequence to the shared Actions
workflow. The `ensign` client and this brief become installable artifacts of
the release rather than files someone copies.
