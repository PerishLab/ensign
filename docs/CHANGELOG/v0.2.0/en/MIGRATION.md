# Migrating to Ensign v0.2.0

No existing deployment requires migration, because v0.2.0 is the first version
distributed through the release lane and no prior version was ever installed
from one.

An operator upgrading a chart installed before this version should know three
things.

The API container image entrypoint changed. It was `["ensign-api", "/ensign"]`,
which pinned the root as the first positional argument and collided with the
new subcommands; it is now `ENTRYPOINT ["ensign-api"]` with
`CMD ["serve", "/ensign"]`. A deployment that overrode the command must pass
`bootstrap` or `serve` explicitly.

Bootstrap moved from a Helm hook Job into an initContainer in the API Pod. The
`-api` ServiceAccount, Role, and RoleBinding that separated the two workloads
are removed, because a single Pod has a single ServiceAccount. The separation
they enforced — a serving process never holding sudo — is now enforced by the
binary, which refuses a `sudo` artifact at runtime by name.

Retained artifacts gained `helm.sh/resource-policy: keep`. Sudo, signing, and
store secrets and the store volume now survive `helm uninstall`. Removing them
is a deliberate act; a reinstall over surviving custody claims the original
estate rather than sealing a new one.

Set `api.pullPolicy: Always` while iterating on a moving image tag. The chart
previously declared no pull policy, so a node kept running the binary it had
already cached under the same tag.
