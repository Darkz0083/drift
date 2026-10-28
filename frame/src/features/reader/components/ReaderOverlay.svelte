<script lang="ts">
  import { commands } from "$shared/ipc/bindings";
  import { extract, type Article } from "../lib/extract";

  let { tab, onclose }: { tab: string; onclose: () => void } = $props();

  // `undefined` while reading, `null` when the page has no article to show.
  let article = $state<Article | null | undefined>(undefined);

  $effect(() => {
    const asked = tab;
    article = undefined;
    void commands
      .pageSource(asked)
      .then((found) => (found ? extract(found.text, found.url) : null))
      .catch(() => null)
      .then((result) => {
        if (asked === tab) article = result;
      });
  });

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window {onkeydown} />

<!-- A view of the page already on screen, drawn over it: nothing about the
     page itself changes, so closing returns exactly where you were. -->
<div class="reader" role="dialog" aria-modal="true" aria-label="Reader view">
  <div class="bar">
    <button class="close" type="button" onclick={onclose}>Close</button>
  </div>
  <div class="sheet">
    {#if article === undefined}
      <p class="state">Reading the page…</p>
    {:else if article === null}
      <p class="state">This page has no article to show.</p>
    {:else}
      <article>
        <h1>{article.title}</h1>
        {#if article.byline}<p class="byline">{article.byline}</p>{/if}
        <p class="count">{article.words} words</p>
        <!-- Sanitised by `extract`: whitelisted tags, no attributes but a safe
             href or src. -->
        <div class="body">{@html article.html}</div>
      </article>
    {/if}
  </div>
</div>

<style>
  .reader {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    flex-direction: column;
    background: var(--surface-page, #fff);
    color: var(--text-primary, #111);
  }
  .bar {
    display: flex;
    justify-content: flex-end;
    padding: 10px 14px;
  }
  .close {
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
    opacity: 0.6;
  }
  .close:hover {
    opacity: 1;
  }
  .sheet {
    flex: 1;
    overflow-y: auto;
    padding: 0 24px 96px;
  }
  article {
    max-width: 34em;
    margin: 0 auto;
  }
  h1 {
    font-size: 2rem;
    line-height: 1.2;
    margin: 0 0 0.4em;
  }
  .byline,
  .count,
  .state {
    opacity: 0.6;
    font-size: 0.875rem;
  }
  .count {
    margin-bottom: 2em;
  }
  .state {
    text-align: center;
    margin-top: 20vh;
  }
  .body :global(p) {
    margin: 0 0 1.1em;
    line-height: 1.65;
  }
  .body :global(h2),
  .body :global(h3) {
    margin: 1.8em 0 0.6em;
    line-height: 1.25;
  }
  .body :global(img) {
    max-width: 100%;
    height: auto;
    margin: 1.4em 0;
  }
  .body :global(blockquote) {
    margin: 1.4em 0;
    padding-left: 1em;
    border-left: 3px solid currentColor;
    opacity: 0.8;
  }
  .body :global(pre) {
    overflow-x: auto;
    padding: 0.8em;
  }
  .body :global(a) {
    color: inherit;
  }
</style>
