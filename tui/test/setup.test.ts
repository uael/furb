import { expect, test } from "bun:test";
import { answering, checks } from "../src/setup.ts";

const providers = [
  { provider: "openai", env: ["OPENAI_API_KEY"], enabled: true, offered: 0 },
  { provider: "anthropic", env: ["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"], enabled: true, offered: 0 },
  { provider: "google", env: ["GOOGLE_API_KEY", "GEMINI_API_KEY"], enabled: true, offered: 0 },
];

test("a machine with no model gives a fix for each check that is missing, and says that no model answers", () => {
  const found = checks({ claude: false, signed: false, providers }, 0);
  expect(found.map(({ label, state }) => [label, state])).toEqual([
    ["Claude command line", "missing"],
    ["Other providers", "missing"],
    ["Local server", "optional"],
    ["Models", "missing"],
  ]);
  expect(found[1]?.fix).toBe(
    "Add a provider with /providers, or edit the config and run /reload. Environment key names: ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_API_KEY.",
  );
  expect(found[3]?.found).toBe("No model answers.");
  expect(found.every((check) => check.fix)).toBe(true);
});

test("a check that stands says what it found, with no fix", () => {
  const ready = providers.map((one) => (one.provider === "openai" ? { ...one, offered: 4 } : one));
  const found = checks(
    {
      claude: true,
      signed: true,
      providers: ready,
      local: { address: "http://localhost:11434", offered: 2 },
    },
    9,
  );
  expect(found.map(({ state, found, fix }) => [state, found, fix])).toEqual([
    ["ready", "Found. Your Claude plan answers.", undefined],
    ["ready", "Models available from openai.", undefined],
    ["ready", "2 models at http://localhost:11434.", undefined],
    ["ready", "9 models answer.", undefined],
  ]);
});

test("a local server that does not answer is missing, with its address", () => {
  const local = { address: "http://127.0.0.1:9", offered: 0, failed: "it listed nothing" };
  const [, , server] = checks({ claude: true, signed: true, providers, local }, 4);
  expect(server).toEqual({
    label: "Local server",
    state: "missing",
    found: "No answer from http://127.0.0.1:9.",
    fix: "Start the server, or set FURB_LOCAL_URL to its address.",
  });
});

test("a claude command line that is not signed in is missing, with the way to sign in and check again", () => {
  const [claude, , , models] = checks({ claude: true, signed: false, providers }, 0);
  expect(claude).toEqual({
    label: "Claude command line",
    state: "missing",
    found: "Found, but not signed in.",
    fix: "Run claude in a terminal, sign in, then press r here to check again.",
  });
  expect(models?.state).toBe("missing");
});

test("the models of a claude command line that is not signed in answer nothing, and the others answer", () => {
  const catalog: [string, string[], number][] = [
    ["claude-cli:opus", [], 1],
    ["claude-cli:sonnet", [], 1],
    ["openai:gpt-5", [], 1],
  ];
  expect(answering({ claude: true, signed: false, providers }, catalog)).toBe(1);
  expect(answering({ claude: true, signed: true, providers }, catalog)).toBe(3);
});

test("a local provider from the config shows its models or asks for a settings repair", () => {
  const configured = { provider: "local", env: [], enabled: true, offered: 1 };
  const ready = checks({ claude: false, signed: false, providers: [...providers, configured] }, 1);
  expect(ready[2]).toEqual({
    label: "Local server",
    state: "ready",
    found: "1 model in the provider settings.",
  });
  const missing = checks(
    { claude: false, signed: false, providers: [...providers, { ...configured, offered: 0 }] },
    0,
  );
  expect(missing[2]).toEqual({
    label: "Local server",
    state: "missing",
    found: "No model from the local provider settings.",
    fix: "Check this provider's URL, model and key in the config, then run /reload.",
  });
});
