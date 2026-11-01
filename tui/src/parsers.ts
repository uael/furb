import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { getTreeSitterClient } from "@opentui/core";

const require = createRequire(import.meta.url);
/** The grammars that the app ships beside those of OpenTUI, so that syntax colors also work offline. Python colors the
 * code of the chain, and each grammar colors a code block of markdown in its language. */
const grammars = ["python", "json"].map((filetype) => ({
  filetype,
  folder: dirname(require.resolve(`tree-sitter-${filetype}/package.json`)),
}));
let ready: { client: ReturnType<typeof getTreeSitterClient>; promise: Promise<void> } | undefined;
export async function loadParsers(): Promise<void> {
  const client = getTreeSitterClient();
  if (ready?.client === client) return ready.promise;
  const promise = (async () => {
    await client.initialize();
    for (const { filetype, folder } of grammars)
      client.addFiletypeParser({
        filetype,
        wasm: join(folder, `tree-sitter-${filetype}.wasm`),
        queries: { highlights: [join(folder, "queries/highlights.scm")] },
      });
    if (!(await client.preloadParser("python"))) throw new Error("The Python syntax parser could not load.");
  })();
  ready = { client, promise };
  await promise;
}
