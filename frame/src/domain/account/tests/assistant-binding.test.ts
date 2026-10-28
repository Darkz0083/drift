import { beforeEach, describe, expect, it, vi } from "vitest";

const workModels = vi.fn();
const workSetModelEndpoint = vi.fn();
const workSetProviderKey = vi.fn();

vi.mock("$shared/ipc/bindings", () => ({
  commands: {
    workModels: (...args: unknown[]) => workModels(...args),
    workSetModelEndpoint: (...args: unknown[]) => workSetModelEndpoint(...args),
    workSetProviderKey: (...args: unknown[]) => workSetProviderKey(...args),
    workClearProviderKey: async () => {},
  },
}));

const SESSION = {
  uid: "u1",
  email: "person@example.com",
  idToken: "id-token",
  refreshToken: "refresh-token",
  expiresAt: Date.now() + 3_600_000,
};

async function fresh() {
  vi.resetModules();
  return await import("../assistant");
}

describe("assistant binding", () => {
  beforeEach(() => {
    workModels.mockReset();
    workSetModelEndpoint.mockReset();
    workSetProviderKey.mockReset();
    // The proxy is the endpoint unless the person chose another.
    workModels.mockResolvedValue({ providers: [{ provider: "compatible", base: null }] });
  });

  it("writes the refresh token, which is the one that does not expire", async () => {
    const { applyToAssistant } = await fresh();
    await applyToAssistant("profile", SESSION);
    expect(workSetProviderKey).toHaveBeenCalledWith("profile", "compatible", "refresh-token");
  });

  it("does nothing the second time for the same session", async () => {
    const { applyToAssistant } = await fresh();
    await applyToAssistant("profile", SESSION);
    await applyToAssistant("profile", SESSION);
    expect(workSetProviderKey).toHaveBeenCalledOnce();
  });

  it("retries after a refused write instead of leaving the assistant without a key", async () => {
    workSetProviderKey.mockRejectedValueOnce(new Error("vault locked"));
    const { applyToAssistant } = await fresh();

    await expect(applyToAssistant("profile", SESSION)).rejects.toThrow();
    // The write is tried again rather than being remembered as done.
    await applyToAssistant("profile", SESSION);
    expect(workSetProviderKey).toHaveBeenCalledTimes(2);
  });

  it("leaves an endpoint the person chose alone", async () => {
    workModels.mockResolvedValue({
      providers: [{ provider: "compatible", base: "https://their.own/v1" }],
    });
    const { applyToAssistant } = await fresh();
    await applyToAssistant("profile", SESSION);
    expect(workSetProviderKey).not.toHaveBeenCalled();
    expect(workSetModelEndpoint).not.toHaveBeenCalled();
  });
});
