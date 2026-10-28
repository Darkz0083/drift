import { describe, expect, it } from "vitest";
import { extract } from "../lib/extract";

const BODY = Array.from(
  { length: 6 },
  (_, index) =>
    `<p>Paragraph ${index} carries enough words to look like prose rather than a caption or a control label.</p>`,
).join("");

const page = (inner: string) =>
  `<!doctype html><html><head><title>Fallback</title></head><body>
    <nav><a href="/">Home</a><a href="/about">About</a></nav>
    <header><h1>Site name</h1></header>
    <article><h1>The real headline</h1><p class="byline">By A Writer</p>${inner}</article>
    <aside>Subscribe to our newsletter</aside>
    <footer>Copyright</footer>
  </body></html>`;

describe("extract", () => {
  it("keeps the article and drops the furniture around it", () => {
    const article = extract(page(BODY), "https://example.com/post");
    expect(article).not.toBeNull();
    expect(article!.title).toBe("The real headline");
    expect(article!.byline).toBe("By A Writer");
    expect(article!.html).toContain("Paragraph 0");
    expect(article!.html).not.toContain("Subscribe to our newsletter");
    expect(article!.html).not.toContain("Copyright");
    expect(article!.words).toBeGreaterThan(40);
  });

  it("refuses a page that is not an article", () => {
    const thin = page("<p>Too short.</p>");
    expect(extract(thin, "https://example.com/")).toBeNull();
  });

  it("removes scripts and refuses javascript: links", () => {
    const hostile = page(
      `${BODY}<script>alert(1)</script><p><a href="javascript:alert(2)">tap</a></p>`,
    );
    const article = extract(hostile, "https://example.com/post")!;
    expect(article.html).not.toContain("<script");
    expect(article.html).not.toContain("javascript:");
  });

  it("resolves relative links against the page", () => {
    const article = extract(
      page(`${BODY}<p><a href="/next">next</a></p>`),
      "https://example.com/post",
    )!;
    expect(article.html).toContain('href="https://example.com/next"');
  });

  it("keeps no attribute but a safe href or src", () => {
    const article = extract(
      page(`${BODY}<p onclick="steal()" style="color:red">plain</p>`),
      "https://example.com/post",
    )!;
    expect(article.html).not.toContain("onclick");
    expect(article.html).not.toContain("style=");
  });
});
