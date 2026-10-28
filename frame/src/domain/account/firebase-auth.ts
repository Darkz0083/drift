/**
 * Firebase email/password auth over the REST API — no SDK, no bundler changes.
 *
 * The API key below is a public project identifier, not a secret: Firebase
 * ships it in every web client. The secrets that matter (Groq, OpenRouter) live
 * on the Cloudflare Worker and never appear here.
 *
 * A session is an ID token (valid ~1 hour) plus a refresh token (valid until
 * revoked). Everything that calls the proxy needs `idToken`, so refresh before
 * it expires — a stale token looks exactly like a broken app to the user.
 */

import { observe } from "$shared/lib/observe";
import { commands } from "$shared/ipc/bindings";

const API_KEY = "AIzaSyBOQ-fMWSK93vIKo3ljo_XYsdAoBBcB1Y8";
const BASE = "https://identitytoolkit.googleapis.com/v1/accounts";
const STORAGE_KEY = "drift.session";

/** The bound the rest of the account domain puts on a request. Without one, a
 *  network that accepts the connection and then says nothing leaves sign-in
 *  and token refresh waiting forever with no way to tell the person. */
const TIMEOUT = 15_000;

/** The message for a request that never answered, or answered too late. */
const unreachable = () => new Error("Couldn't reach the sign-in service. Check your connection and try again.");

export type Session = {
  uid: string;
  email: string;
  idToken: string;
  refreshToken: string;
  /** Epoch ms. Refresh when within 5 minutes of this. */
  expiresAt: number;
};

type AuthResponse = {
  localId?: string;
  email?: string;
  idToken?: string;
  refreshToken?: string;
  expiresIn?: string;
  error?: { message?: string };
};

/** Firebase's error codes are shouty constants; say something a person can read. */
function readable(code: string | undefined): string {
  switch (code) {
    case "EMAIL_EXISTS":
      return "That email already has an account. Try signing in.";
    case "EMAIL_NOT_FOUND":
    case "INVALID_PASSWORD":
    case "INVALID_LOGIN_CREDENTIALS":
      return "Wrong email or password.";
    case "WEAK_PASSWORD":
      return "Password must be at least 6 characters.";
    case "INVALID_EMAIL":
      return "That doesn't look like an email address.";
    case "TOO_MANY_ATTEMPTS_TRY_LATER":
      return "Too many attempts. Wait a minute and try again.";
    case "CONFIGURATION_NOT_FOUND":
      return "Email sign-in isn't enabled for this project yet.";
    case "OPERATION_NOT_ALLOWED":
      return "Email sign-in isn't enabled for this project yet.";
    default:
      return "Something went wrong. Try again.";
  }
}

async function call(path: string, body: unknown): Promise<Session> {
  const observed = await observe(
    fetch(`${BASE}:${path}?key=${API_KEY}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    }),
    TIMEOUT,
  );
  if (observed.state !== "received") throw unreachable();
  const res = observed.value;
  const data = (await res.json()) as AuthResponse;

  if (!res.ok || data.error) {
    throw new Error(readable(data.error?.message));
  }

  const session: Session = {
    uid: data.localId ?? "",
    email: data.email ?? "",
    idToken: data.idToken ?? "",
    refreshToken: data.refreshToken ?? "",
    expiresAt: Date.now() + Number(data.expiresIn ?? 3600) * 1000,
  };
  save(session);
  return session;
}

export function signUp(email: string, password: string): Promise<Session> {
  return call("signUp", { email, password, returnSecureToken: true });
}

export function signIn(email: string, password: string): Promise<Session> {
  return call("signInWithPassword", { email, password, returnSecureToken: true });
}

/** Exchange the refresh token for a fresh ID token. */
export async function refreshSession(session: Session): Promise<Session> {
  if (!session.refreshToken) throw new Error("Session expired. Sign in again.");

  // The secure-token endpoint, not identitytoolkit: this is the only one that
  // mints a new ID token from a refresh token.
  const observed = await observe(
    fetch(`https://securetoken.googleapis.com/v1/token?key=${API_KEY}`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: `grant_type=refresh_token&refresh_token=${encodeURIComponent(session.refreshToken)}`,
    }),
    TIMEOUT,
  );
  if (observed.state !== "received") throw unreachable();
  const res = observed.value;
  if (!res.ok) throw new Error("Session expired. Sign in again.");

  const data = (await res.json()) as {
    id_token?: string;
    refresh_token?: string;
    expires_in?: string;
    user_id?: string;
  };

  const fresh: Session = {
    uid: data.user_id ?? session.uid,
    email: session.email,
    idToken: data.id_token ?? "",
    refreshToken: data.refresh_token ?? session.refreshToken,
    expiresAt: Date.now() + Number(data.expires_in ?? 3600) * 1000,
  };
  if (!fresh.idToken) throw new Error("Session expired. Sign in again.");
  save(fresh);
  return fresh;
}

/** True when the token is close enough to expiry to be worth refreshing. */
export function needsRefresh(session: Session): boolean {
  return Date.now() > session.expiresAt - 5 * 60 * 1000;
}

/** Held in memory so `load` stays synchronous, as its callers expect. */
let cached: Session | null = null;
let hydrated = false;
/** One read at a time: several surfaces ask at startup. */
let reading: Promise<void> | null = null;

function parse(raw: string | null): Session | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Session;
    return parsed.idToken ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * Reads the stored session once, from the Keychain.
 *
 * A copy left in `localStorage` by an earlier version is migrated: it is only
 * dropped once the Keychain write is confirmed, so a refused write leaves the
 * person signed in rather than silently signed out.
 *
 * A read that fails is not a read that found nothing. Marking this done on
 * failure would present a locked vault as "signed out" for the rest of the
 * launch; leaving it undone lets the next caller try again.
 */
export function hydrate(): Promise<void> {
  if (hydrated) return Promise.resolve();
  if (reading) return reading;
  reading = (async () => {
    let stored: string | null;
    try {
      stored = await commands.sessionGet();
    } catch {
      return;
    }
    const found = parse(stored);
    if (found) {
      cached = found;
      hydrated = true;
      return;
    }
    const legacy = parse(localStorage.getItem(STORAGE_KEY));
    if (!legacy) {
      hydrated = true;
      return;
    }
    cached = legacy;
    const kept = await commands.sessionSet(JSON.stringify(legacy)).catch(() => false);
    if (kept) localStorage.removeItem(STORAGE_KEY);
    hydrated = true;
  })().finally(() => {
    reading = null;
  });
  return reading;
}

export function load(): Session | null {
  return cached;
}

export function save(session: Session): void {
  cached = session;
  void commands.sessionSet(JSON.stringify(session)).catch(() => {});
}

export function clear(): void {
  cached = null;
  void commands.sessionClear().catch(() => {});
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* A storage that refuses is one less copy to worry about. */
  }
}

/** The proxy endpoint this session is entitled to. */
export const PROXY_BASE = "https://drift-proxy.kolesovl848.workers.dev/v1";
