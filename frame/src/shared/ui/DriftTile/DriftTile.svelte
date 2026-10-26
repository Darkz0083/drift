<script lang="ts">
  /** Drift's app icon itself, its own squircle and shadow included, the
   *  same in either theme. */
  let { size = 16 }: { size?: number } = $props();
</script>

<img
  class="drift-tile"
  src="/drift-icon.png"
  srcset="/drift-icon.png 128w, /drift-icon@2x.png 256w"
  sizes={`${size}px`}
  width={size}
  height={size}
  alt=""
  aria-hidden="true"
  draggable="false"
/>

<style>
  .drift-tile {
    display: inline-block;
    flex: none;
    object-fit: contain;
  }
</style>
