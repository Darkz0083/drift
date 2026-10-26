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

const API_KEY = "AIzaSyBOQ-fMWSK93vIKo3ljo_XYsdAoBBcB1Y8";
const BASE = "https://identitytoolkit.googleapis.com/v1/accounts";
const STORAGE_KEY = "drift.session";

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
  const res = await fetch(`${BASE}:${path}?key=${API_KEY}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
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
  const res = await fetch(
    `https://securetoken.googleapis.com/v1/token?key=${API_KEY}`,
    {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: `grant_type=refresh_token&refresh_token=${encodeURIComponent(session.refreshToken)}`,
    }
  );
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

export function load(): Session | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Session;
    return parsed.idToken ? parsed : null;
  } catch {
    return null;
  }
}

export function save(session: Session): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
}

export function clear(): void {
  localStorage.removeItem(STORAGE_KEY);
}

/** The proxy endpoint this session is entitled to. */
export const PROXY_BASE = "https://drift-proxy.kolesovl848.workers.dev/v1";
