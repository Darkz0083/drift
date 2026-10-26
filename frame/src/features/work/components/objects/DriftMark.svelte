<script module lang="ts">
  /** Whether a name is Drift itself, which has no site mark to borrow. */
  export const isDrift = (name: string) => /^drift\b/iu.test(name.trim());
</script>

<script lang="ts">
  import DriftTile from "$shared/ui/DriftTile";

  let { size = 16 }: { size?: number } = $props();
</script>

<DriftTile {size} />
