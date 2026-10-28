<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";
  import Button from "$shared/ui/Button";
  import { sidebarTree } from "$features/tabs";
  import { essentialIcons } from "$domain/essential-icons.svelte";
  import PreferenceSelect from "../PreferenceSelect.svelte";
  import PreferenceSwitch from "../PreferenceSwitch.svelte";

  // The kept sites, in the order the rail shows them.
  let essentials = $derived(
    sidebarTree.favorites.flatMap((entry) => (entry.kind === "tab" ? [entry.tab] : [])),
  );
  $effect(() => {
    void essentialIcons.load(essentials.map((tab) => tab.id));
  });
</script>

<SettingsGroup title={m.tabs_group_opening()}
  ><PreferenceSelect id="tabs.new-position" preference="tabs.new-position" /><PreferenceSelect
    id="tabs.external"
    preference="tabs.external-position"
  /><PreferenceSwitch id="tabs.duplicates" preference="tabs.switch-to-open" /><PreferenceSwitch
    id="tabs.automatic-pip"
    preference="tabs.automatic-pip"
  /></SettingsGroup
>
<SettingsGroup title={m.tabs_group_closing()}
  ><PreferenceSelect id="tabs.close" preference="tabs.after-close" /><PreferenceSelect
    id="tabs.close-button"
    preference="tabs.close-button"
  /></SettingsGroup
>
<SettingsGroup title={m.tabs_group_archive()}
  ><PreferenceSelect id="tabs.archive" preference="tabs.archive" /></SettingsGroup
>
{#if essentials.length > 0}<SettingsGroup title={m.pref_essentials_icons()}
    >{#each essentials as tab (tab.id)}<SettingsRow
        settingId={`essentials.icon.${tab.id}`}
        title={tab.title || m.untitled_tab()}
        description={essentialIcons.image(tab.id)
          ? m.pref_essentials_icon_custom()
          : m.pref_essentials_icon_site()}
        ><div class="icon-actions">
          <Button size="compact" onclick={() => void essentialIcons.choose(tab.id)}
            >{m.pref_ntp_background_choose()}</Button
          >
          {#if essentialIcons.image(tab.id)}<Button
              size="compact"
              variant="ghost"
              onclick={() => void essentialIcons.clear(tab.id)}
              >{m.pref_ntp_background_remove()}</Button
            >{/if}
        </div></SettingsRow
      >{/each}</SettingsGroup
  >{/if}

<style>
  .icon-actions {
    display: flex;
    gap: 8px;
  }
</style>
