# ensign

Ensign closes the identity-authentication domain over Keel's resource
hotspots: enrollment, credentials, sessions, recovery, API keys, OIDC,
forward-auth, signing, and their API and CLI ceremonies. This brief is the
authority on how a deployment is brought up and operated. Nothing in `docs/`
overrides it; ensign's only reader is the agent operating it.

## Upstream

Repository: https://git.perish.top/PerishLab/ensign

Report defects, missing shapes, and unclear guidance there as issues. The
laws of the engine live in `keel:docs/*`; this brief obeys them from the
outside and never retells them. An engine gap becomes a keel issue and a
registry release, never a local workaround.

## Principles

**Bootstrap is an operation, not a resource.** It is an explicit startup
ceremony with a beginning and an end. It is never a row, a role, a remote
route, or a long-running mode.

**Possession is not identity.** Sudo is estate genesis possession. It may open
the first invitation and authorize the ordinary grant that makes the first
administrator, but it is never converted into an account or retained as a
steady-state role.

**Custody and runtime are separate seats.** The process that may mint or keep
sudo is not the process that serves requests. When orchestration cannot hold
that line, the binary holds it.

**Refusal over repair.** A missing prerequisite is a named refusal and the
listener never opens. Conflicting or duplicate state is drift and is never
silently rewritten.

**The cold start is vendor-free.** Password primitive, recovery codes, and the
sudo window are the floor. Every layer's last exit depends only on lower-layer
possession, never on an external vendor.

## Laws

### Command

The server image owns two process commands:

```text
api bootstrap ROOT --artifact NAME=DEST ...
api serve ROOT --artifact NAME=DEST ...
```

`serve` remains the default for local compatibility but never falls through to
bootstrap. The remote `ensign` client does not gain database access; its
sudo-authorized first-admin command is an ordinary Keel resource operation over
the normal product boundary.

### Artifacts

One named artifact and one caller-selected destination. A destination adapter
may load, durably keep, and later read bytes; its file, webhook, Kubernetes, or
vault behavior is outside both domain closures.

| Name | Meaning owner | Bootstrap | Runtime |
|------|---------------|-----------|---------|
| `sudo` | Keel estate genesis | read or mint, keep, then seal | forbidden |
| `signing` | Ensign OIDC signing | read or generate, then keep | read-only |

The shared grammar does not create a generic secret domain. The two artifacts
have separate destinations, retention, mounts, and access policy. An adapter
never exposes either value in logs, arguments, status, or errors.

### Ceremony

Bootstrap runs in one fixed order:

```text
resolve independent artifact destinations
  -> open store and Keel bootstrap surface
  -> load sudo, or mint and durably keep it
  -> transactionally seal or exactly replay the estate
  -> find or create the canonical service Actor
  -> explicitly sow Keel-gate and Ensign service grants
  -> load and validate signing, or generate and durably keep it
  -> verify every runtime prerequisite
  -> exit success
```

The service Actor is exactly one live `Actor` with login and name `ensign`,
kind `svc`, and `barred=false`. Absence permits creation under bootstrap sudo.

Service grants are ordinary `@grant` rows. Explicit bootstrap may idempotently
restore the declared baseline; ordinary startup never does.

### Signing

Provision has three states: absent generates one P-256 private key and durably
keeps canonical PKCS#8 PEM; valid loads it and preserves the exact bytes and
derived `kid`; malformed or inaccessible refuses without overwrite.

Generation and persistence are bootstrap operations. Runtime only parses the
configured read-only artifact. Signing never becomes a Keel row, and sudo never
becomes a signing input.

### Runtime

Ordinary `api serve` performs only these checks before listening:

1. Keel ordinary bind succeeds on a catalogued estate.
2. The canonical service Actor exists and is usable.
3. Every Keel-gate and Ensign service grant in the declared baseline is live.
4. The signing artifact is readable and valid.
5. A deployable candidate has an explicit stable issuer.

It creates no Actor, grant, gate seed, key, directory, or custody artifact.
**It refuses a `sudo` artifact by name.**

### Replay

Bootstrap is resumable across every commit edge:

| Durable state | Replay |
|---------------|--------|
| no artifact, no estate | mint, keep, seal |
| sudo kept, no estate | reuse sudo and seal |
| estate sealed, service absent | exact replay, then create service |
| service present, grants partial | idempotently sow baseline |
| grants ready, signing absent | generate and keep signing |
| all ready | verify and exit without semantic change |

An estate with missing sudo custody cannot be bootstrapped by minting a new
value. Concurrent contenders rely on Keel's estate winner rule and idempotent
resource writes; different sudo values conflict fail-closed. Signing
destination creation is atomic, so only one key becomes durable and losers
reload and verify the winner.

### Kubernetes lifecycle

Bootstrap runs as an **initContainer in the API Pod**, not as a Job. A Helm
post-install hook cannot serve here: `helm install --wait` waits for resources
to become ready before running the hook, so an API waiting on bootstrap and a
bootstrap Job waiting on API readiness deadlock and the Job is never created.
An initContainer makes the ordering structural.

The consequence is load-bearing: one Pod has one ServiceAccount, so **RBAC no
longer separates custody from runtime**. The binary is the remaining seat, and
`crates/api/tests/bootstrap.rs::forbidden` is what keeps it honest.

Every retained artifact carries `helm.sh/resource-policy: keep`. Sudo, signing,
and store secrets plus the store volume survive uninstall, and a reinstall
claims the original estate rather than minting a new one. The other face of
`keep` is that deleting them is a deliberate human act.

A generated credential is generated **once and reused**, never called twice for
the same secret. `helm lint` and `helm template` are both silent about a
template that calls `randAlphaNum` in two places; only a real install fails.

The chart deploys no web plane.

### First administrator

Bootstrap creates no human, Invite, admin role, or wildcard human grant. After
the API is ready an operator uses sudo custody through the `ensign` CLI to open
the first Invite, joins as an ordinary Actor, then creates one ordinary
site-admin grant. Sudo returns to dormant custody for lockout recovery.

## Standing

Mechanized, in this repository's own test suite:

- `crates/api/tests/law.rs::grant` — the literal `"@grant"` appears nowhere in
  ensign sources; grant construction is Keel vocabulary and is never retold.
- `crates/api/tests/law.rs::ambient` — `.sudo()` appears only in
  `api/src/door/mod.rs`, the identity-birth exception.
- `crates/api/tests/bootstrap.rs::forbidden` — `serve` refuses a `sudo`
  artifact by name, with the estate sealed and signing valid so that no other
  refusal can stand in for it.
- `crates/api/tests/bootstrap.rs::{replay, occupied, malformed, severed,
  contended}` — the replay table and the concurrent-contender rule.

Prose-only; no checker will stop a violation:

- the ceremony order and the service-grant baseline;
- the separation of sudo and signing custody, mounts, and lifecycle;
- `resource-policy: keep` on every retained artifact;
- generating a credential once per secret;
- refusing to convert sudo into an account or a steady-state role.

Repository shape and release law belong to Plumb; load the `plumb` skill rather
than restating its clauses here.

## Invocation

```bash
api bootstrap /ensign --artifact sudo=file:/run/sudo --artifact signing=file:/run/sign.pem
api bootstrap /ensign --artifact sudo=kubernetes:ensign-sudo --artifact signing=kubernetes:ensign-signing
api serve /ensign --artifact signing=kubernetes:ensign-signing

ensign invite [note]                  # sudo custody opens the first Invite
ensign join <code> <login> <name>     # the first human becomes an ordinary Actor
ensign login <login>                  # password from stdin
ensign crown <login>                  # one ordinary site-admin grant, sudo from stdin
```

The binary is the truth about its own flags: prefer `api --help` and
`ensign --help` over assuming.
