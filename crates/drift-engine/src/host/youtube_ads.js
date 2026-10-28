// Removes YouTube's ads before the player can use them.
//
// YouTube serves in-video ads from its own origin, so no network rule can stop
// them: the page itself decides to play one. This runs at document start in the
// page's world and does the two things a filter list cannot:
//
//   1. Scrubs the ad fields of `ytInitialPlayerResponse` as the page assigns
//      it, so the player is never handed an ad to play.
//   2. Removes the ad renderers YouTube re-inserts as you browse, and presses
//      the skip control when one appears anyway.
//
// It touches nothing else: no network interception, no storage or cookie
// access, no data leaves the page.
(() => {
  "use strict";
  if (window.__driftYoutubeAds) return;
  Object.defineProperty(window, "__driftYoutubeAds", { value: true });

  const AD_KEYS = ["adPlacements", "playerAds", "adSlots", "adBreakHeartbeatParams"];

  const scrub = (response) => {
    if (!response || typeof response !== "object") return response;
    for (const key of AD_KEYS) {
      if (key in response) {
        try {
          delete response[key];
        } catch {
          response[key] = undefined;
        }
      }
    }
    return response;
  };

  // The player reads this global as it is assigned, so the value is scrubbed
  // in flight rather than after the player already holds it.
  let playerResponse;
  try {
    Object.defineProperty(window, "ytInitialPlayerResponse", {
      configurable: true,
      get: () => playerResponse,
      set: (value) => {
        playerResponse = scrub(value);
      },
    });
  } catch {
    // A page that made the property non-configurable keeps its own value.
  }

  const SELECTORS = [
    "#player-ads",
    "#masthead-ad",
    "ytd-ad-slot-renderer",
    "ytd-display-ad-renderer",
    "ytd-promoted-sparkles-web-renderer",
    "ytd-promoted-video-renderer",
    "ytd-in-feed-ad-layout-renderer",
    "ytd-banner-promo-renderer",
    "ytd-statement-banner-renderer",
    "ytd-companion-slot-renderer",
    ".ytp-ad-overlay-container",
    ".ytp-ad-progress-list",
    ".ytp-ad-text-overlay",
  ].join(",");

  const SKIP = ".ytp-ad-skip-button, .ytp-skip-ad-button, .ytp-ad-skip-button-modern";

  const remove = (root) => {
    if (root instanceof Element) {
      if (root.matches(SELECTORS)) root.remove();
      for (const node of root.querySelectorAll(SELECTORS)) node.remove();
    } else if (root === document) {
      for (const node of document.querySelectorAll(SELECTORS)) node.remove();
    }
    const skip = document.querySelector(SKIP);
    if (skip instanceof HTMLElement) skip.click();
  };

  const start = () => {
    remove(document);
    new MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) remove(node);
      }
    }).observe(document.documentElement, { childList: true, subtree: true });
  };

  if (document.documentElement) start();
  else document.addEventListener("DOMContentLoaded", start, { once: true });
})();
