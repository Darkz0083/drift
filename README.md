# Drift

**A fork of [Zephium](https://github.com/zephium-browser/Zephium).**

Drift is Zephium, rebranded and rebuilt for a different purpose: a browser with
its own identity, its own model access, and accounts that don't depend on
anyone else's service.

Everything that makes it a browser — the Rust engine, WebKit on macOS and
WebView2 on Windows, the ad blocker, the extension host, the tab and space
model — is Zephium's work, and it is excellent. This fork changes the surface
and the account layer, and nothing else.

---

## What's different from upstream

**Branding.** New name, new wordmark, new icon, new app identity. The wordmark is
set in Anurati and converted to outlines, so it ships as geometry rather than
depending on an installed font.

**Accounts.** Upstream's account system talks to Zephium's own service. Drift
replaces it with:

- **Firebase** for identity — email and password, no third party in the loop
- **A small Cloudflare Worker** that holds the model provider keys server-side
  and meters usage per account

The point of that design: a person signs up and gets the assistant without ever
entering an API key. The keys stay on the server; the client only ever holds a
session token.

> **Status:** the account page signs in against Firebase, and the Worker answers
> requests correctly. Wiring sign-in so it configures the model provider
> automatically is not finished yet — until it is, the assistant needs its
> provider configured by hand on each machine.

---

## Build

Prerequisites: Rust (the toolchain in `rust-toolchain.toml`), Node, and the
platform's own web engine.

```bash
npm install --prefix frame
npm install --prefix desktop

npm run build --prefix frame
cd desktop && npx tauri build
```

The macOS bundle lands in `target/debug/bundle/macos/Drift.app`. Windows
installers are built by CI — `.github/workflows/windows.yml` runs on
`windows-latest` and uploads an NSIS `.exe` and an `.msi` as artifacts. Tauri
cannot cross-compile to Windows from macOS, so CI (or a Windows machine) is the
only route.

Two build notes learned the hard way: **move `frame/dist` aside before
rebuilding** (vite clears it, and some environments block bulk deletes), and the
app occasionally launches to a blank window once — a relaunch paints it.

---

## Licence

Inherited from upstream: **Mozilla Public License 2.0**. See `LICENSE`.

MPL is file-level copyleft. If you distribute a binary built from this source,
you must make the source of the MPL-covered files available. The bundled
EasyList and EasyPrivacy filter lists are used under CC BY-SA 3.0; see
`assets/blocker-seed/v1/`.

## Credit

The overwhelming majority of this codebase is the work of the Zephium project and
its contributors. If you want the browser without the fork, use
[upstream](https://github.com/zephium-browser/Zephium) — it is the original, and
it is actively maintained.
