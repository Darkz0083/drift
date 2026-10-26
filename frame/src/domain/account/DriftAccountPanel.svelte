<script lang="ts">
  /**
   * Drift account — sign up / sign in against Firebase, then hand the session
   * to the caller so it can point the assistant at the proxy.
   *
   * Deliberately self-contained: it imports only `firebase-auth` and Svelte's
   * own runes, so mounting it can't drag in the Rust-backed account domain.
   */
  import {
    clear,
    load,
    needsRefresh,
    refreshSession,
    signIn,
    signUp,
    type Session,
  } from "./firebase-auth";

  let {
    onsession,
  }: {
    /** Called with a live session on sign-in, and with null on sign-out. */
    onsession?: (session: Session | null) => void;
  } = $props();

  let session = $state<Session | null>(load());
  let email = $state("");
  let password = $state("");
  let busy = $state(false);
  let error = $state("");
  let mode = $state<"in" | "up">("in");

  // Keep the token fresh while the panel is mounted; a stale token is
  // indistinguishable from a broken app.
  $effect(() => {
    if (!session) return;
    const timer = setInterval(async () => {
      if (session && needsRefresh(session)) {
        try {
          session = await refreshSession(session);
          onsession?.(session);
        } catch {
          session = null;
          clear();
          onsession?.(null);
        }
      }
    }, 60_000);
    return () => clearInterval(timer);
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    error = "";
    busy = true;
    try {
      const next =
        mode === "up" ? await signUp(email.trim(), password) : await signIn(email.trim(), password);
      session = next;
      password = "";
      onsession?.(next);
    } catch (e) {
      error = e instanceof Error ? e.message : "Something went wrong.";
    } finally {
      busy = false;
    }
  }

  function signOut() {
    clear();
    session = null;
    onsession?.(null);
  }
</script>

<div class="drift-account">
  {#if session}
    <div class="who">
      <div class="avatar" aria-hidden="true">{(session.email[0] ?? "?").toUpperCase()}</div>
      <div class="who-text">
        <strong>{session.email}</strong>
        <span>Signed in — the assistant is ready</span>
      </div>
    </div>
    <button class="ghost" onclick={signOut}>Sign out</button>
  {:else}
    <div class="tabs">
      <button class:on={mode === "in"} onclick={() => (mode = "in")}>Sign in</button>
      <button class:on={mode === "up"} onclick={() => (mode = "up")}>Create account</button>
    </div>

    <form onsubmit={submit}>
      <label>
        <span>Email</span>
        <input
          type="email"
          bind:value={email}
          autocomplete="email"
          placeholder="you@example.com"
          required
        />
      </label>
      <label>
        <span>Password</span>
        <input
          type="password"
          bind:value={password}
          autocomplete={mode === "up" ? "new-password" : "current-password"}
          placeholder={mode === "up" ? "at least 6 characters" : "••••••••"}
          required
        />
      </label>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <button class="primary" type="submit" disabled={busy || !email || !password}>
        {busy ? "Working…" : mode === "up" ? "Create account" : "Sign in"}
      </button>
    </form>

    <p class="note">
      An account is only needed for the assistant. Browsing works signed out.
    </p>
  {/if}
</div>

<style>
  .drift-account {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 380px;
    color: var(--color-text, #1c1c1f);
    font: 13px/1.5 -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 10px;
    background: var(--color-fill, rgb(0 0 0 / 0.05));
  }
  .tabs button {
    flex: 1;
    padding: 6px 10px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--color-muted, rgb(28 28 31 / 0.62));
    font: inherit;
    cursor: default;
  }
  .tabs button.on {
    background: var(--color-surface, #fff);
    color: var(--color-text, #1c1c1f);
    font-weight: 550;
    box-shadow: 0 0 0 0.5px rgb(0 0 0 / 0.08), 0 1px 2px rgb(0 0 0 / 0.06);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  label span {
    color: var(--color-muted, rgb(28 28 31 / 0.62));
    font-size: 12px;
  }
  input {
    padding: 8px 10px;
    border: 0;
    border-radius: 10px;
    background: var(--color-field, rgb(228 228 230 / 0.7));
    color: inherit;
    font: inherit;
    outline: none;
  }
  input:focus {
    box-shadow: inset 0 0 0 1px var(--color-field-ring, rgb(0 0 0 / 0.14));
  }

  .primary {
    margin-top: 4px;
    padding: 9px 14px;
    border: 0;
    border-radius: 10px;
    background: var(--color-lit, #222226);
    color: var(--color-on-lit, #fff);
    font: inherit;
    font-weight: 550;
    cursor: default;
  }
  .primary:disabled {
    opacity: 0.4;
  }

  .ghost {
    align-self: flex-start;
    padding: 7px 12px;
    border: 0;
    border-radius: 9px;
    background: var(--color-fill, rgb(0 0 0 / 0.05));
    color: inherit;
    font: inherit;
    cursor: default;
  }

  .error {
    margin: 0;
    color: var(--color-danger, #c0474f);
    font-size: 12px;
  }

  .note {
    margin: 0;
    color: var(--color-faint, rgb(28 28 31 / 0.42));
    font-size: 11.5px;
    line-height: 1.55;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .avatar {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--color-fill, rgb(0 0 0 / 0.05));
    font-weight: 550;
  }
  .who-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .who-text strong {
    font-weight: 550;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who-text span {
    color: var(--color-muted, rgb(28 28 31 / 0.62));
    font-size: 12px;
  }
</style>
