// Linux builds do not compile yet, so stop before Cargo with a clear note.
if (process.platform === "linux") {
  console.error(
    "Drift isn't available on Linux yet. We're working on it; follow https://drift.app for news.",
  );
  process.exit(1);
}
