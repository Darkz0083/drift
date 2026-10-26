<script lang="ts">
  /**
   * The Account page.
   *
   * The upstream account system talked to Zephium's own service, which doesn't
   * exist for this build. This replaces it with Drift's own: Firebase for
   * identity, and a Cloudflare Worker holding the model keys, so a signed-in
   * person gets the assistant without ever entering a key.
   */
  import DriftAccountPanel from "$domain/account/DriftAccountPanel.svelte";
</script>

<section class="drift-account-page">
  <header>
    <h1>Account</h1>
    <p>
      Sign in to use the assistant. Browsing, tabs, history and everything else
      in Drift work without an account.
    </p>
  </header>

  <DriftAccountPanel />
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
