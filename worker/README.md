# Drift's model proxy

The Worker the app calls for AI models. It holds the provider keys so the app
never does, and speaks the OpenAI shape the app already uses.

## Contract

| Endpoint | Method | Notes |
| --- | --- | --- |
| `/v1/models` | GET | Both catalogues, namespaced by provider |
| `/v1/chat/completions` | POST | Streams when the upstream streams |

`Authorization: Bearer <Firebase token>` is required on both.

**Two token kinds are accepted, on purpose.** An **ID token** is verified
through Identity Toolkit, which checks signature, audience and expiry in one
call — it lives an hour, so a leak ages out. A **refresh token** is exchanged
for an ID token first. The second path exists only so the app can move to ID
tokens without a flag day; it is not the target state, because a refresh token
is long-lived and the exchange rotates it behind the client's back.

Models are namespaced and the namespace picks the route: `openrouter/…` goes to
OpenRouter, `groq/…` to Groq. An unnamespaced model is treated as Groq's, which
is what the previous proxy served. `/v1/models` returns the same names, so what
the app offers is exactly what this Worker can route.

## Deploying

Needs a Cloudflare account that owns `drift-proxy`.

```sh
cd worker
npm install
npx wrangler login                      # once, interactive
npx wrangler secret put GROQ_API_KEY
npx wrangler secret put OPENROUTER_API_KEY
npx wrangler deploy
```

`FIREBASE_API_KEY` is a variable in `wrangler.toml` — it is a public project
identifier, not a secret. The two provider keys are secrets and must never be
committed.

## Switching the app to ID tokens

Once this is deployed, the client change is one line in
`frame/src/domain/account/assistant.ts`:

```ts
const token = session.idToken || session.refreshToken;   // was the other way round
```

The Worker accepts either, so the flip is safe to make separately from the
deploy — and reverting it is equally safe.

## What this does not do

- No per-user rate limiting. Cloudflare's own rate limiting rules are the place
  for that, not application code.
- No usage accounting or billing.
- No streaming-specific handling beyond passing the upstream body through.
