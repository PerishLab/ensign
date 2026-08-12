# Restrained scenarios

## Bootstrap must run under Kubernetes

Bootstrap is an initContainer in the API Pod, never a Job. `helm install --wait`
waits for readiness before running a hook, so an API waiting on bootstrap and a
Job waiting on API readiness deadlock and the Job is never created. The
consequence is load-bearing: one Pod has one ServiceAccount, so RBAC no longer
separates custody from runtime and the binary is the remaining seat.

## Signing material may already exist

Absent generates one P-256 private key and durably keeps canonical PKCS#8 PEM;
valid loads it and preserves the exact bytes and derived `kid`; malformed or
inaccessible refuses without overwrite. Generation is a bootstrap operation, and
sudo never becomes a signing input.

## Two bootstraps contend

Different sudo values conflict fail-closed, and an estate whose sudo custody is
missing cannot be bootstrapped by minting a new value. Signing destination
creation is atomic, so losers reload and verify the winner.
