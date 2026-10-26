# Security

## The engine

Almost all of this codebase — the browser engine, the blocker, the extension
host, the IPC surface — belongs to
[Zephium](https://github.com/zephium-browser/Zephium). A vulnerability there
affects them first and their fix is the one worth having. **Report engine issues
to upstream**, privately, per
[their security policy](https://github.com/zephium-browser/Zephium/blob/main/SECURITY.md).

## The fork's own surface

Drift adds two things that can fail on their own:

**The account layer.** Firebase email/password auth, and a Cloudflare Worker
that holds model provider keys. If you find a way to read another account's
usage, bypass the daily quota, or reach a provider key from the client, that is a
real bug — please report it privately rather than in an issue.

**The build pipeline.** The Windows workflow and the packaging scripts.

## What is *not* a vulnerability

- The client holds a Firebase session token. It is meant to. It expires, and it
  grants only what that account can already do.
- The Worker refuses requests without a valid token. That is the design.
- Free-tier provider rate limits surfacing as errors. That is the providers, not
  a flaw here.

## Reporting

Open a [private security advisory](https://github.com/Darkz0083/drift/security/advisories/new)
on this repository. Please don't open a public issue for anything exploitable.
