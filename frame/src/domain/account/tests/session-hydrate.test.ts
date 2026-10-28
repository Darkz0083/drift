import { beforeEach, describe, expect, it, vi } from "vitest";

const sessionGet = vi.fn<() => Promise<string | null>>();
const sessionSet = vi.fn<(value: string) => Promise<boolean>>();

vi.mock("$shared/ipc/bindings", () => ({
  commands: {
    sessionGet: () => sessionGet(),
    sessionSet: (value: string) => sessionSet(value),
    sessionClear: async () => true,
  },
}));

const SESSION = {
  uid: "u1",
  email: "person@example.com",
  idToken: "id-token",
  refreshToken: "refresh-token",
  expiresAt: Date.now() + 3_600_000,
};

/** The unit project runs in node, so the web storage the migration reads is
 *  stood up here rather than assumed. */
function stubStorage() {
  const items = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => void items.set(key, value),
    removeItem: (key: string) => void items.delete(key),
    clear: () => items.clear(),
  });
  return items;
}

/** Each test starts from a module that has not read anything yet. */
async function fresh() {
  vi.resetModules();
  stubStorage();
  return await import("../firebase-auth");
}

describe("session hydration", () => {
  beforeEach(() => {
    sessionGet.mockReset();
    sessionSet.mockReset();
  });

  it("reads the stored session out of the keychain", async () => {
    sessionGet.mockResolvedValue(JSON.stringify(SESSION));
    const auth = await fresh();
    await auth.hydrate();
    expect(auth.load()).toEqual(SESSION);
  });

  it("reports signed out when nothing is stored", async () => {
    sessionGet.mockResolvedValue(null);
    const auth = await fresh();
    await auth.hydrate();
    expect(auth.load()).toBeNull();
  });

  it("a failed read is retried rather than reported as signed out for good", async () => {
    sessionGet.mockRejectedValueOnce(new Error("vault locked"));
    const auth = await fresh();

    await auth.hydrate();
    expect(auth.load()).toBeNull();

    // The vault answers on the next attempt: the session must still arrive.
    sessionGet.mockResolvedValue(JSON.stringify(SESSION));
    await auth.hydrate();
    expect(auth.load()).toEqual(SESSION);
  });

  it("migrates a session left in local storage and drops it once kept", async () => {
    sessionGet.mockResolvedValue(null);
    sessionSet.mockResolvedValue(true);
    const auth = await fresh();
    localStorage.setItem("drift.session", JSON.stringify(SESSION));

    await auth.hydrate();
    expect(auth.load()).toEqual(SESSION);
    expect(sessionSet).toHaveBeenCalledOnce();
    expect(localStorage.getItem("drift.session")).toBeNull();
  });

  it("keeps the old copy when the keychain refuses the write", async () => {
    sessionGet.mockResolvedValue(null);
    sessionSet.mockResolvedValue(false);
    const auth = await fresh();
    localStorage.setItem("drift.session", JSON.stringify(SESSION));

    await auth.hydrate();
    expect(auth.load()).toEqual(SESSION);
    // Dropping it here would sign the person out on the next launch.
    expect(localStorage.getItem("drift.session")).not.toBeNull();
  });

  it("reads once, however many callers ask", async () => {
    sessionGet.mockResolvedValue(JSON.stringify(SESSION));
    const auth = await fresh();
    await Promise.all([auth.hydrate(), auth.hydrate(), auth.hydrate()]);
    expect(sessionGet).toHaveBeenCalledOnce();
  });
});
