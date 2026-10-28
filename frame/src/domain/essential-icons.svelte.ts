import { commands } from "$shared/ipc/bindings";
import { toneOf, type IconTone } from "$domain/favicons/favicons.svelte";

/**
 * Marks people chose for their kept sites, by tab id.
 *
 * The rail draws marks as 32 square pixels, not as URLs, so a chosen picture is
 * decoded once into the same shape a favicon arrives in — and judged by the
 * same tone rule, so it sits against the chrome the way every other mark does.
 */
const SIDE = 32;

type Mark = { image: ImageData; tone: IconTone };

let marks = $state<Record<string, Mark>>({});

/** A `data:` URL as the pixels the rail draws, scaled to the mark's size. */
async function decode(url: string): Promise<Mark | null> {
  const picture = new Image();
  picture.src = url;
  try {
    await picture.decode();
  } catch {
    return null;
  }
  const canvas = document.createElement("canvas");
  canvas.width = SIDE;
  canvas.height = SIDE;
  const context = canvas.getContext("2d");
  if (!context) return null;
  context.drawImage(picture, 0, 0, SIDE, SIDE);
  const image = context.getImageData(0, 0, SIDE, SIDE);
  return { image, tone: toneOf(image) };
}

async function store(id: string, url: string): Promise<Mark | null> {
  const mark = await decode(url);
  if (mark) marks = { ...marks, [id]: mark };
  return mark;
}

export const essentialIcons = {
  /** The chosen mark, or null when the site's own favicon should be used. */
  mark: (id: string): Mark | null => marks[id] ?? null,

  /** Reads the stored marks for the essentials on screen. */
  async load(ids: string[]) {
    await Promise.all(
      ids.map(async (id) => {
        if (marks[id]) return;
        const url = await commands.essentialIconCurrent(id).catch(() => null);
        if (url) await store(id, url);
      }),
    );
  },

  /** Asks for a picture. Null when the picker was dismissed or refused it. */
  async choose(id: string): Promise<Mark | null> {
    const result = await commands.essentialIconChoose(id).catch(() => null);
    if (!result || result.status !== "ok" || !result.data) return null;
    return store(id, result.data);
  },

  async clear(id: string) {
    await commands.essentialIconClear(id).catch(() => undefined);
    const { [id]: _removed, ...rest } = marks;
    marks = rest;
  },
};
