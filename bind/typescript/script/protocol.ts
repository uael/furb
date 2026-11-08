import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compile } from "json-schema-to-typescript";

const root = join(import.meta.dir, "..", "..", "..");
export const schemaPath = join(root, "host", "protocol.schema.json");
export const typesPath = join(root, "bind", "typescript", "src", "protocol.ts");

/** The types of the protocol, as the JSON Schema that the crate furb-host gives makes them. */
export async function generated(): Promise<string> {
  const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
  const banner = `/* The types of the one protocol of the host, made from host/protocol.schema.json by
 * bind/typescript/script/protocol.ts: run bun run protocol after a change of the protocol. */`;
  const types = await compile(schema, "Protocol", {
    bannerComment: banner,
    additionalProperties: false,
    format: false,
  });
  // The structs of the verbs of a view close their fields, so the schema inlines them in their verb: each keeps its name
  // here, as the types that a skill and a window import.
  const views = { Mount: "view.mount", Tree: "view.tree", ViewEvent: "view.event", ViewEffect: "view.effect", Dispose: "view.dispose" };
  const aliases = Object.entries(views)
    .map(([name, verb]) => `export type ${name} = Omit<Extract<Request1, { verb: "${verb}" }>, "verb">;`)
    .join("\n");
  const made = `${types.trimEnd()}\n\n${aliases}\n\nexport const PROTOCOL = ${Number(schema["x-protocol"])};\n`;
  const biome = join(root, "node_modules", ".bin", "biome");
  const formatted = Bun.spawnSync([biome, "format", `--stdin-file-path=${typesPath}`], {
    cwd: root,
    stdin: Buffer.from(made),
  });
  if (formatted.exitCode !== 0) throw new Error(`biome cannot format the types: ${formatted.stderr}`);
  return formatted.stdout.toString();
}

if (import.meta.main) {
  writeFileSync(typesPath, await generated());
}
