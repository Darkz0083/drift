<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences } from "$domain/preferences";
  import { commands } from "$shared/ipc/bindings";
  import Button from "$shared/ui/Button";
  import Select from "$shared/ui/Select";
  import Switch from "$shared/ui/Switch";
  import SettingsGroup from "$shared/ui/SettingsGroup";
  import SettingsRow from "$shared/ui/SettingsRow";

  let greeting = $derived(preferences.value("ui.newtab-greeting") === "true");
  let clock = $derived(preferences.value("ui.newtab-clock") === "true");

  // Held as a data URL rather than a path: the page never learns where the
  // file lives, and no filesystem protocol has to be open to show it.
  let background = $state<string | null>(null);
  let backgroundFailed = $state(false);
  $effect(() => {
    void commands.newtabBackgroundCurrent().then((url) => (background = url ?? null));
  });
  async function chooseBackground() {
    backgroundFailed = false;
    try {
      const url = await commands.newtabBackgroundChoose();
      if (url) background = url;
    } catch {
      backgroundFailed = true;
    }
  }
  async function removeBackground() {
    await commands.newtabBackgroundClear();
    background = null;
  }

  const set = (key: preferences.PreferenceKey, value: string) => void preferences.set(key, value);
</script>

<SettingsGroup title={m.settings_newtab_content()}>
  <SettingsRow
    settingId="ntp.greeting"
    title={m.pref_ntp_greeting()}
    description={m.pref_ntp_greeting_help()}
    ><Switch
      label={m.pref_ntp_greeting()}
      labelHidden
      checked={greeting}
      disabled={preferences.saving()}
      onchange={(v) => set("ui.newtab-greeting", String(v))}
    /></SettingsRow
  >
  <SettingsRow
    settingId="ntp.personalize"
    title={m.pref_ntp_personalize()}
    description={m.pref_ntp_personalize_help()}
    ><Switch
      label={m.pref_ntp_personalize()}
      labelHidden
      checked={preferences.value("ui.newtab-name") === "true"}
      disabled={preferences.saving() || !greeting}
      onchange={(v) => set("ui.newtab-name", String(v))}
    /></SettingsRow
  >
  <SettingsRow
    settingId="ntp.clock"
    title={m.pref_ntp_clock()}
    description={m.pref_ntp_clock_help()}
    ><Switch
      label={m.pref_ntp_clock()}
      labelHidden
      checked={clock}
      disabled={preferences.saving()}
      onchange={(v) => set("ui.newtab-clock", String(v))}
    /></SettingsRow
  >
  <SettingsRow
    settingId="ntp.clock-format"
    title={m.pref_ntp_clock_format()}
    description={m.pref_ntp_clock_format_help()}
    ><Select
      label={m.pref_ntp_clock_format()}
      labelHidden
      value={preferences.value("ui.newtab-clock-format")}
      disabled={preferences.saving() || !clock}
      options={[
        { value: "system", label: m.pref_ntp_clock_format_option_0() },
        { value: "12h", label: m.pref_ntp_clock_format_option_1() },
        { value: "24h", label: m.pref_ntp_clock_format_option_2() },
      ]}
      onchange={(v) => set("ui.newtab-clock-format", v)}
    /></SettingsRow
  >
  {#if preferences.enabled("tasks")}<SettingsRow
      settingId="ntp.tasks"
      title={m.pref_ntp_tasks()}
      description={m.pref_ntp_tasks_help()}
      ><Switch
        label={m.pref_ntp_tasks()}
        labelHidden
        checked={preferences.value("ui.newtab-tasks") === "true"}
        disabled={preferences.saving()}
        onchange={(v) => set("ui.newtab-tasks", String(v))}
      /></SettingsRow
    >{/if}
  <SettingsRow
    settingId="ntp.background"
    title={m.pref_ntp_background()}
    description={backgroundFailed ? m.pref_ntp_background_failed() : m.pref_ntp_background_help()}
    ><div class="background">
      <Button size="compact" onclick={() => void chooseBackground()}
        >{m.pref_ntp_background_choose()}</Button
      >
      {#if background}<Button size="compact" variant="ghost" onclick={() => void removeBackground()}
          >{m.pref_ntp_background_remove()}</Button
        >{/if}
    </div></SettingsRow
  >
</SettingsGroup>

<style>
  .background {
    display: flex;
    gap: 8px;
  }
</style>
