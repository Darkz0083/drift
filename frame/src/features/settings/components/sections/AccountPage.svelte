<script lang="ts">
  /**
   * The Account page.
   *
   * The upstream account system talked to Drift's own service, which doesn't
   * exist for this build. This replaces it with Drift's own: Firebase for
   * identity, and a Cloudflare Worker holding the model keys, so a signed-in
   * person gets the assistant without ever entering a key.
   *
   * The session is handed to the assistant here, not inside the panel: the
   * panel knows nothing about profiles or model providers, and this page is
   * where both the profile and the account meet.
   */
  import DriftAccountPanel from "$domain/account/DriftAccountPanel.svelte";
  import { applyToAssistant } from "$domain/account/assistant";
  import { load, type Session } from "$domain/account/firebase-auth";
  import { tabs } from "$domain/tabs";

  let profile = $derived(tabs.profile());

  /** Only a real profile can own model settings; incognito keeps none. */
  function assistantProfile(): string | null {
    const current = profile;
    return current && current.kind !== "incognito" ? current.id : null;
  }

  function hand(session: Session | null) {
    const id = assistantProfile();
    if (id) void applyToAssistant(id, session);
  }

  // A session can already be on disk from an earlier launch: the panel starts
  // from it without announcing it, so point the assistant at it here.
  $effect(() => {
    if (!assistantProfile()) return;
    hand(load());
  });
</script>

<section class="drift-account-page">
  <header>
    <h1>Account</h1>
    <p>
      Sign in to use the assistant. Browsing, tabs, history and everything else
      in Drift work without an account.
    </p>
  </header>

  <DriftAccountPanel onsession={hand} />
</section>

<style>
  .drift-account-page {
    display: flex;
    flex-direction: column;
    gap: 26px;
    max-width: 560px;
    padding: 4px 2px 40px;
  }

  header {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 550;
    letter-spacing: -0.02em;
    color: var(--color-text);
  }

  header p {
    margin: 0;
    max-width: 440px;
    color: var(--color-muted);
    font-size: 12.5px;
    line-height: 1.6;
  }
</style>
