# Branch Protection

Nightmare treats `main` as the stable release branch and `dev` as the active
integration branch.

## main

`main` requires pull request review before merge. It also requires the
`CI / rust` status check to pass before merge, requires branches to be up to
date, dismisses stale reviews, requires code owner review, requires last-push
approval, requires conversation resolution, applies to admins, prevents force
pushes, and prevents branch deletion.

## dev

`dev` requires CI before merge, but does not require review. This keeps the
integration branch fast enough for development while still preventing broken
code from becoming the shared base. Force pushes and branch deletion are
disabled, conversation resolution is required, and protection applies to admins.

## Release Tags

Version tags matching `v*.*.*` trigger the release workflow. The workflow
verifies the tag is on `origin/main`, runs locked workspace tests, builds release
binaries, uploads artifacts, and publishes a GitHub release for tagged versions.

## Reconciliation

Before applying this policy, local `main` and `origin/main` were checked and
were aligned. The local `main` branch tracks `origin/main`.
