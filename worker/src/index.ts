/**
 * Drift's model proxy.
 *
 * Holds the provider keys so the app never does, and answers the OpenAI shape
 * the app speaks: `GET /v1/models` and `POST /v1/chat/completions`.
 *
 * Authentication is a Firebase token in `Authorization: Bearer …`. Both kinds
 * are accepted:
 *
 *   - an **ID token**, verified through Identity Toolkit, which checks the
 *     signature, the audience and the expiry in one call. This is the one to
 *     prefer: it lives an hour, so a leak ages out on its own.
 *   - a **refresh token**, exchanged for an ID token first. This exists so the
 *     client can move over without a flag day — it is not the target state.
 *
 * Models are namespaced by provider, and the namespace decides the route:
 * `openrouter/…` goes to OpenRouter, `groq/…` to Groq. `/v1/models` returns the
 * same names, so what the app offers is exactly what this Worker can route.
 */

interface Env {
  GROQ_API_KEY: string;
  OPENROUTER_API_KEY: string;
  FIREBASE_API_KEY: string;
}

const GROQ = "https://api.groq.com/openai/v1";
const OPENROUTER = "https://openrouter.ai/api/v1";

/** A request larger than this is refused before it reaches a provider. */
const MAX_BODY_BYTES = 1_000_000;

/** Nothing here waits on a provider forever. */
const UPSTREAM_TIMEOUT_MS = 120_000;

const JSON_HEADERS = { "content-type": "application/json; charset=utf-8" };

function problem(status: number, message: string): Response {
  return new Response(JSON.stringify({ error: { message, type: "drift_proxy" } }), {
    status,
    headers: JSON_HEADERS,
  });
}

/**
 * The caller's verified identity, or null.
 *
 * A refresh token is exchanged; an ID token is checked. Either way what comes
 * back is an ID token, so downstream code never has to care which was sent.
 */
async function identify(env: Env, bearer: string): Promise<string | null> {
  const looksLikeIdToken = bearer.split(".").length === 3;

  if (looksLikeIdToken) {
    const response = await fetch(
      `https://identitytoolkit.googleapis.com/v1/accounts:lookup?key=${env.FIREBASE_API_KEY}`,
      {
        method: "POST",
        headers: JSON_HEADERS,
        body: JSON.stringify({ idToken: bearer }),
      },
    );
    if (!response.ok) return null;
    const data = (await response.json()) as { users?: unknown[] };
    return Array.isArray(data.users) && data.users.length > 0 ? bearer : null;
  }

  // A refresh token is long-lived and single-use; exchanging it also rotates it,
  // which the client cannot see. That is another reason to move to ID tokens.
  const response = await fetch(
    `https://securetoken.googleapis.com/v1/token?key=${env.FIREBASE_API_KEY}`,
    {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: `grant_type=refresh_token&refresh_token=${encodeURIComponent(bearer)}`,
    },
  );
  if (!response.ok) return null;
  const data = (await response.json()) as { id_token?: string };
  return data.id_token ?? null;
}

/** Where a model lives, and its name there. */
function route(model: string): { base: string; model: string } {
  const slash = model.indexOf("/");
  if (slash > 0) {
    const provider = model.slice(0, slash);
    const rest = model.slice(slash + 1);
    if (provider === "openrouter") return { base: OPENROUTER, model: rest };
    if (provider === "groq") return { base: GROQ, model: rest };
  }
  // An unnamespaced model is a Groq one, which is what the old proxy served.
  return { base: GROQ, model };
}

function keyFor(env: Env, base: string): string {
  return base === OPENROUTER ? env.OPENROUTER_API_KEY : env.GROQ_API_KEY;
}

/** Both catalogues, under the names this Worker routes. */
async function models(env: Env): Promise<Response> {
  const fetchList = async (base: string, key: string, prefix: string) => {
    if (!key) return [];
    const response = await fetch(`${base}/models`, {
      headers: { authorization: `Bearer ${key}` },
      signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
    });
    if (!response.ok) return [];
    const data = (await response.json()) as { data?: { id?: string }[] };
    return (data.data ?? [])
      .filter((entry): entry is { id: string } => typeof entry.id === "string")
      .map((entry) => ({
        id: `${prefix}/${entry.id}`,
        object: "model",
        owned_by: prefix,
      }));
  };

  const [groq, openrouter] = await Promise.all([
    fetchList(GROQ, env.GROQ_API_KEY, "groq"),
    fetchList(OPENROUTER, env.OPENROUTER_API_KEY, "openrouter"),
  ]);
  return new Response(JSON.stringify({ object: "list", data: [...groq, ...openrouter] }), {
    headers: JSON_HEADERS,
  });
}

async function chat(env: Env, request: Request): Promise<Response> {
  const length = Number(request.headers.get("content-length") ?? "0");
  if (length > MAX_BODY_BYTES) return problem(413, "That request is too large.");

  const raw = await request.text();
  if (raw.length > MAX_BODY_BYTES) return problem(413, "That request is too large.");

  let body: Record<string, unknown>;
  try {
    body = JSON.parse(raw) as Record<string, unknown>;
  } catch {
    return problem(400, "That request was not JSON.");
  }
  if (typeof body.model !== "string") return problem(400, "A model is required.");

  const { base, model } = route(body.model);
  const key = keyFor(env, base);
  if (!key) return problem(503, `No key is configured for ${body.model}.`);

  const upstream = await fetch(`${base}/chat/completions`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      authorization: `Bearer ${key}`,
      // OpenRouter attributes traffic by these; Groq ignores them.
      "http-referer": "https://drift.app",
      "x-title": "Drift",
    },
    body: JSON.stringify({ ...body, model }),
    signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
  });

  // Pass the answer through as it arrives: a streamed completion has to stay
  // streamed, and the headers carry the content type the client switches on.
  return new Response(upstream.body, {
    status: upstream.status,
    headers: {
      "content-type": upstream.headers.get("content-type") ?? "application/json",
      "cache-control": "no-store",
    },
  });
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "access-control-allow-origin": "*",
          "access-control-allow-headers": "authorization, content-type",
          "access-control-allow-methods": "GET, POST, OPTIONS",
        },
      });
    }

    const authorization = request.headers.get("authorization") ?? "";
    const bearer = authorization.startsWith("Bearer ") ? authorization.slice(7).trim() : "";
    if (!bearer) return problem(401, "Sign in to use Drift's models.");
    if (!(await identify(env, bearer))) return problem(401, "That session is no longer valid.");

    if (url.pathname === "/v1/models" && request.method === "GET") return models(env);
    if (url.pathname === "/v1/chat/completions" && request.method === "POST") {
      return chat(env, request);
    }
    return problem(404, "No such endpoint.");
  },
};
