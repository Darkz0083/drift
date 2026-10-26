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
 */
import { commands } from "$shared/ipc/bindings";
import { PROXY_BASE, type Session } from "./firebase-auth";

/** The token last written, so a refresh that changes nothing writes nothing. */
let applied: string | null = null;
/** The profile the token was written for; the Keychain item is global. */
let bound: string | null = null;

/**
 * Points `profile`'s assistant at the proxy with `session`, or back at nothing
 * when the session is gone. Safe to call on every change: an unchanged token
 * does no work.
 */
export async function applyToAssistant(profile: string, session: Session | null): Promise<void> {
  if (!profile) return;

  if (!session) {
    if (bound === null) return;
    const previous = bound;
    applied = null;
    bound = null;
    await commands.workClearProviderKey(previous, "compatible").catch(() => {});
    return;
  }

  const token = session.refreshToken || session.idToken;
  if (!token || (applied === token && bound === profile)) return;
  applied = token;
  bound = profile;

  // The endpoint first: the key is validated against it, so validating before
  // the address is set would check the wrong server.
  await commands.workSetModelEndpoint(profile, PROXY_BASE);
  await commands.workSetProviderKey(profile, "compatible", token);
}

/** Forgets what was written, for tests and for a profile switch. */
export function resetAssistantBinding(): void {
  applied = null;
  bound = null;
}
