# Contributing

**This is a personal fork of [Zephium](https://github.com/zephium-browser/Zephium).**
It is not the upstream project, and changes here are not reviewed by Drift's
maintainer.

## If you want to improve the browser

Go upstream. Nearly all of this code is theirs, and a fix made here helps nobody
but this fork. Open your issue or pull request at
[zephium-browser/Zephium](https://github.com/zephium-browser/Zephium).

## If you want to change something Drift-specific

That means the fork's own surface: branding, the account layer (Firebase +
the Cloudflare proxy), and the build pipeline. Open an issue describing the
change before writing much code — the account layer is small and deliberate, and
it is easy to add something that duplicates what is already there.

## Practical notes

- `npm`, not `pnpm`. The lockfile is pnpm's but the CI and local builds use npm;
  keep both working if you touch dependencies.
- Move `frame/dist` aside before rebuilding — vite clears it, and some
  environments block bulk deletes.
- The first Windows build takes ~40 minutes; after that the cargo cache makes it
  a few.
