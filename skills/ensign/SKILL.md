---
name: ensign
description: Operate Ensign, the identity product built on Keel. Use when bringing a deployment up, running or debugging bootstrap, handling the sudo and signing artifacts, creating the first administrator, deploying the Helm chart, or changing the Ensign repository.
---

# Ensign

Ensign closes identity authentication over Keel's resource hotspots: enrollment,
credentials, sessions, recovery, API keys, OIDC, forward-auth, and signing. This
brief is the authority on bringing a deployment up and operating it. Outside the
Ensign repository or deployment it is silent.

## Objects

- A **bootstrap** is one startup ceremony with a beginning and an end, never a
  row, a role, a route, or a long-running mode.
- An **artifact** is one named secret with one caller-selected destination:
  `sudo` for Keel estate genesis, `signing` for the OIDC key.
- **Sudo** is genesis possession; it is never converted into an account or
  retained as a steady-state role.
- The **service Actor** is exactly one live `Actor` named `ensign`, kind `svc`,
  `barred=false`, holding ordinary Keel `@grant` rows.

## Actions

```bash
api bootstrap ROOT --artifact NAME=DEST ...
api serve ROOT --artifact NAME=DEST ...
ensign invite [note]
ensign join <code> <login> <name>
ensign login <login>
ensign crown <login>
```

The binary is the authority for flags; prefer `api --help` and `ensign --help`.

## Operating laws

- Custody and runtime are separate seats. The process that may mint or keep sudo
  is not the process that serves requests; when orchestration cannot hold that
  line, the binary holds it.
- Refusal over repair. A missing prerequisite is a named refusal and the listener
  never opens; duplicate or conflicting state is drift, never silently rewritten.
- The cold start is vendor-free. The password primitive, recovery codes, and the
  sudo window are the floor, and no layer's last exit depends on a vendor.
- Generate a credential once per secret and reuse it. `helm lint` and
  `helm template` are both silent about a second `randAlphaNum` call.
- Every retained artifact carries `helm.sh/resource-policy: keep`, so deleting
  one is a deliberate human act. The chart deploys no web plane.
- The engine's laws live in `keel:docs/*` and are never retold here. An engine
  gap becomes a Keel issue and a registry release, never a local workaround.
- `crates/api/tests/{law,bootstrap}.rs` mechanize grant vocabulary, the `.sudo()`
  exception, `serve` refusing a `sudo` artifact, and the replay table. Run the
  suite instead of restating its verdicts.
- Repository shape, dependency policy, and landing belong to Plumb; structural
  and vocabulary law to Ectropy. Never claim either as an Ensign check.
- Report defects at https://git.perish.top/PerishLab/ensign.

Use [PATHS.md](PATHS.md) for routine flows and [SCENARIOS.md](SCENARIOS.md) only
when one of its bounded cases applies.
