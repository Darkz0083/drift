/**
 * The bridge between a Drift account and the assistant's model providers.
 *
 * Signing in has to do something: the assistant calls the proxy, the proxy
 * wants a Firebase token, and Rust is the only thing that can hold a
 * credential. So a session is pushed into the `compatible` provider — the
 * OpenAI-compatible endpoint the person would otherwise configure by hand.
 *
 * The **refresh token** is stored, not the ID token. An ID token dies after an
 * hour, and a dead token looks exactly like a broken app; the refresh token
 * does not expire, and the proxy mints its own ID tokens from it. One write
 * therefore lasts until the person signs out, across launches, with no timer.
 *
 * The endpoint is Drift's own unless the person set one. Their server always
 * wins: signing in never repoints an endpoint they chose, and never touches a
 * key that belongs to it.
 */
import { commands } from "$shared/ipc/bindings";
import { PROXY_BASE, type Session } from "./firebase-auth";

/** The token last written, so a refresh that changes nothing writes nothing. */
let applied: string | null = null;
/** The profile the token was written for; the Keychain item is global. */
let bound: string | null = null;

/** The endpoint the assistant would call now, if it can be read. */
async function currentBase(profile: string): Promise<string | null> {
  const view = await commands.workModels(profile).catch(() => null);
  return view?.providers.find((row) => row.provider === "compatible")?.base ?? null;
}

/**
 * Points `profile`'s assistant at the proxy with `session`, or back at nothing
 * when the session is gone. Safe to call on every change: an unchanged token
 * does no work, and a server the person configured is left alone.
 */
export async function applyToAssistant(profile: string, session: Session | null): Promise<void> {
  if (!profile) return;

  // Rust ships Drift's endpoint as the default, so an unset one reads back as
  // the proxy. Anything else is a server the person chose.
  const base = await currentBase(profile);
  if (base && base !== PROXY_BASE) return;

  if (!session) {
    if (bound === null) return;
    const previous = bound;
    applied = null;
    bound = null;
    await commands.workClearProviderKey(previous, "compatible").catch(() => {});
    return;
  }

  // The refresh token, not the ID token. An ID token dies after an hour and
  // nothing renews it unless the account panel happens to be open — the only
  // refresh loop lives in that lazily-mounted component. A dead token is
  // indistinguishable from a broken app, so the credential handed to a
  // long-lived consumer has to be the one that does not expire. The proxy
  // mints its own ID tokens from this one.
  const token = session.refreshToken || session.idToken;
  if (!token || (applied === token && bound === profile)) return;

  // The endpoint first: the key is validated against it, so validating before
  // the address is set would check the wrong server.
  //
  // Recorded only once both writes land. Marking the token applied first meant
  // a refused write — a locked vault, a store busy with an import — left the
  // assistant with no key and no retry until the next launch, which reads as
  // "AI is broken" rather than as a write that failed once.
  try {
    await commands.workSetModelEndpoint(profile, PROXY_BASE);
    await commands.workSetProviderKey(profile, "compatible", token);
  } catch (error) {
    applied = null;
    bound = null;
    throw error;
  }
  applied = token;
  bound = profile;
}

/** Forgets what was written, for tests and for a profile switch. */
export function resetAssistantBinding(): void {
  applied = null;
  bound = null;
}
