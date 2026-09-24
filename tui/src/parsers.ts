import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { getTreeSitterClient } from "@opentui/core";

const require = createRequire(import.meta.url);
const python = dirname(require.resolve("tree-sitter-python/package.json"));
// Ship the Python grammar with the app, so syntax colors also work offline.
let ready: { client: ReturnType<typeof getTreeSitterClient>; promise: Promise<void> } | undefined;
export async function loadParsers(): Promise<void> {
  const client = getTreeSitterClient();
  if (ready?.client === client) return ready.promise;
  const promise = (async () => {
    await client.initialize();
    client.addFiletypeParser({
      filetype: "python",
      wasm: join(python, "tree-sitter-python.wasm"),
      queries: { highlights: [join(python, "queries/highlights.scm")] },
    });
    if (!(await client.preloadParser("python"))) throw new Error("The Python syntax parser could not load.");
  })();
  ready = { client, promise };
  await promise;
}
