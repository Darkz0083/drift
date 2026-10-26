import { describe, expect, it } from "vitest";
import {
  contrastOf,
  decodeShare,
  encodeShare,
  onAccent,
  parseTheme,
  preset,
  presets,
  resolveTheme,
  serializeTheme,
} from "../palette";

const nord = resolveTheme("nord", "").spec;

describe("theme storage", () => {
  it("names every preset once, Drift first", () => {
    expect(presets[0]!.id).toBe("drift");
    expect(new Set(presets.map((entry) => entry.id)).size).toBe(presets.length);
  });

  it("round-trips a custom theme and refuses anything core would refuse", () => {
    expect(parseTheme(serializeTheme(nord))).toEqual(nord);
    const text = serializeTheme(nord);
    for (const invalid of [
      text.replace("#eceff4", "#ECEFF4"),
      text.replace("#eceff4", "#eceff"),
      text.replace(/}$/, ',"contrast":50}'),
      text.replace('"from":"nord"', '"from":"custom"'),
      text.replace("}", ',"css":"x"}'),
      "[]",
      "",
      text + " ".repeat(400),
    ])
      expect(parseTheme(invalid)).toBeNull();
  });

  it("resolves a broken or unknown choice to Drift", () => {
    expect(resolveTheme("custom", "{").id).toBe("drift");
    expect(resolveTheme("monokai", "").id).toBe("drift");
    expect(resolveTheme("custom", serializeTheme(nord))).toMatchObject({ custom: true });
  });

  it("shares a theme as one pasteable string", () => {
    const shared = encodeShare({ spec: nord, contrast: 64 });
    expect(shared).toMatch(/^drift-theme:v1:[A-Za-z0-9_-]+$/);
    expect(decodeShare(`  ${shared}\n`)).toEqual({ spec: nord, contrast: 64 });
    const share = (body: string) => `drift-theme:v1:${btoa(body)}`;
    expect(decodeShare(share(serializeTheme(nord)))).toBeNull();
    for (const contrast of ["101", "6.5", '"50"'])
      expect(
        decodeShare(share(serializeTheme(nord).replace(/}$/, `,"contrast":${contrast}}`))),
      ).toBeNull();
    for (const invalid of ["", "drift-theme:v1:", "drift-theme:v2:abc", shared + "!"])
      expect(decodeShare(invalid)).toBeNull();
    expect(decodeShare(`drift-theme:v1:${btoa('{"light":1}')}`)).toBeNull();
  });

  it("reads a stored contrast strictly", () => {
    expect(contrastOf("0")).toBe(0);
    expect(contrastOf("100")).toBe(100);
    for (const text of ["", "101", "050", "5.5", "-1"]) expect(contrastOf(text)).toBe(50);
  });

  it("sets ink on an accent that reads against it", () => {
    expect(onAccent("#0969da")).toBe("#ffffff");
    expect(onAccent(preset("nord")!.dark.accent)).not.toBe("#ffffff");
  });
});
