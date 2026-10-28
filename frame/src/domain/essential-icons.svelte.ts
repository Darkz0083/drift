import { commands } from "$shared/ipc/bindings";

/**
 * Marks people chose for their kept sites, by tab id.
 *
 * Absent means the site's own favicon, which is the ordinary case. The image is
 * a `data:` URL rather than a path: the page never learns where the file lives.
 */
let marks = $state<Record<string, string>>({});

export const essentialIcons = {
  image: (id: string): string | null => marks[id] ?? null,

  /** Reads the stored marks for the essentials on screen. */
  async load(ids: string[]) {
    const found: Record<string, string> = {};
    await Promise.all(
      ids.map(async (id) => {
        const url = await commands.essentialIconCurrent(id).catch(() => null);
        if (url) found[id] = url;
      }),
    );
    marks = { ...marks, ...found };
  },

  /** Asks for an image. Returns null when the picker was dismissed. */
  async choose(id: string): Promise<string | null> {
    const url = await commands.essentialIconChoose(id).catch(() => null);
    if (url) marks = { ...marks, [id]: url };
    return url;
  },

  async clear(id: string) {
    await commands.essentialIconClear(id).catch(() => undefined);
    const { [id]: _removed, ...rest } = marks;
    marks = rest;
  },
};
