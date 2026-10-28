/**
 * Turns a page's HTML into the article it contains.
 *
 * This is deliberately not a full Readability: it scores the obvious
 * containers, keeps a whitelist of block tags, and drops every attribute
 * except a link's `href` and an image's `src`/`alt`. The output is re-rendered
 * by the app's own typography, so anything that survives here is trusted to be
 * inert markup — no scripts, no event handlers, no `javascript:` URLs.
 */

export type Article = {
  title: string;
  byline: string | null;
  html: string;
  words: number;
};

/** Removed before anything is scored: none of it is article text. */
const DROP = [
  "script",
  "style",
  "noscript",
  "iframe",
  "object",
  "embed",
  "form",
  "button",
  "input",
  "select",
  "textarea",
  "svg",
  "canvas",
  "nav",
  "header",
  "footer",
  "aside",
  "template",
  "[role=navigation]",
  "[role=banner]",
  "[role=complementary]",
  "[aria-hidden=true]",
].join(",");

/** Kept as-is. Everything else is unwrapped, keeping its text. */
const KEEP = new Set([
  "P",
  "H1",
  "H2",
  "H3",
  "H4",
  "H5",
  "H6",
  "UL",
  "OL",
  "LI",
  "BLOCKQUOTE",
  "PRE",
  "CODE",
  "FIGURE",
  "FIGCAPTION",
  "IMG",
  "A",
  "EM",
  "STRONG",
  "B",
  "I",
  "BR",
  "HR",
  "TABLE",
  "THEAD",
  "TBODY",
  "TR",
  "TD",
  "TH",
]);

const CANDIDATES = ["article", "main", "[role=main]", "body"];

const textLength = (node: Element) => (node.textContent ?? "").replace(/\s+/g, " ").trim().length;

/** A safe absolute URL, or null when the scheme is not one a page may load. */
function safeUrl(value: string, base: string): string | null {
  try {
    const url = new URL(value, base);
    return url.protocol === "http:" || url.protocol === "https:" || url.protocol === "data:"
      ? url.href
      : null;
  } catch {
    return null;
  }
}

/** Keeps whitelisted tags, strips every attribute but a safe `href`/`src`. */
function sanitize(root: Element, base: string) {
  for (const node of [...root.querySelectorAll(DROP)]) node.remove();
  for (const element of [...root.querySelectorAll("*")]) {
    if (!KEEP.has(element.tagName)) {
      element.replaceWith(...element.childNodes);
      continue;
    }
    const href = element.tagName === "A" ? element.getAttribute("href") : null;
    const src = element.tagName === "IMG" ? element.getAttribute("src") : null;
    const alt = element.tagName === "IMG" ? element.getAttribute("alt") : null;
    for (const attribute of [...element.attributes]) element.removeAttribute(attribute.name);
    if (href) {
      const safe = safeUrl(href, base);
      if (safe) element.setAttribute("href", safe);
    }
    if (src) {
      const safe = safeUrl(src, base);
      if (safe) element.setAttribute("src", safe);
      else element.remove();
      if (alt) element.setAttribute("alt", alt);
    }
  }
  // Anything left with no text and no picture is noise.
  for (const element of [...root.querySelectorAll("p,li,figure,blockquote")]) {
    if (!element.textContent?.trim() && !element.querySelector("img")) element.remove();
  }
}

export function extract(html: string, url: string): Article | null {
  if (typeof DOMParser === "undefined") return null;
  const doc = new DOMParser().parseFromString(html, "text/html");

  let best: Element | null = null;
  let bestLength = 0;
  for (const selector of CANDIDATES) {
    for (const candidate of doc.querySelectorAll(selector)) {
      const length = textLength(candidate);
      // The body is only a fallback: it always contains everything else.
      if (length > bestLength && (selector !== "body" || best === null)) {
        best = candidate;
        bestLength = length;
      }
    }
    if (best && selector !== "body") break;
  }
  if (!best || bestLength < 200) return null;

  const title =
    (best.querySelector("h1")?.textContent ?? doc.querySelector("h1")?.textContent ?? doc.title ?? "")
      .replace(/\s+/g, " ")
      .trim() || "Untitled";

  const byline = [
    "[rel=author]",
    ".byline",
    ".author",
    "[class*=byline]",
    "[class*=author]",
  ]
    .map((selector) => doc.querySelector(selector)?.textContent?.replace(/\s+/g, " ").trim())
    .find((value) => value && value.length <= 120);

  const article = best.cloneNode(true) as Element;
  sanitize(article, url);
  const text = (article.textContent ?? "").replace(/\s+/g, " ").trim();
  if (text.length < 200) return null;

  return {
    title,
    byline: byline ?? null,
    html: article.innerHTML,
    words: text.split(" ").filter(Boolean).length,
  };
}
