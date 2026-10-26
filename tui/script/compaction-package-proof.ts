import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import type { Parts as SourceParts } from "../src/parts.ts";

const home = resolve("tui/.furb/tests");
await mkdir(home, { recursive: true });
const directory = await mkdtemp(join(home, "compaction-package-"));
try {
  const bundle = join(directory, "parts.mjs");
  const built = await Bun.build({
    entrypoints: [resolve("tui/src/parts.ts")],
    target: "bun",
    external: ["furb", "furb/life"],
    outdir: directory,
    naming: "parts.mjs",
  });
  if (!built.success) throw new AggregateError(built.logs, "The Parts bundle failed.");
  const { Parts } = (await import(bundle)) as { Parts: typeof SourceParts };
  const custom = join(directory, "custom.ts");
  await writeFile(custom, 'export default furb => furb.command("custom", "A project command", () => {});');
  let active: string[] = [];
  const parts = new Parts(
    directory,
    () => {},
    () => [{ name: "custom", file: custom }],
    async () => active,
  );
  const ensure = (value: unknown, message: string) => {
    if (!value) throw new Error(message);
  };
  try {
    await parts.load();
    ensure(!parts.command("compact"), "A disabled built-in was bundled as active.");
    ensure(parts.command("custom"), "The bundle lost the project part.");
    active = ["compaction"];
    await parts.load();
    const command = parts.command("compact");
    ensure(command, "The enabled built-in is missing from the bundle.");
    const sent: string[] = [];
    await command?.run('Keep "exact".\nNext step.', {
      chain: "chain3",
      acts: [],
      of: () => [],
      say: () => {},
      run: async (word) => {
        sent.push(word);
      },
    });
    ensure(
      sent.length === 1 && sent[0] === 'compact(summary="Keep \\"exact\\".\\nNext step.")',
      "The bundled command changed the existing extension call.",
    );
    await parts.load();
    ensure(
      parts.loaded.flatMap((part) => part.commands).filter((one) => one.name === "compact").length === 1,
      "Reload duplicated the built-in command.",
    );
    active = [];
    await parts.load();
    ensure(
      !parts.command("compact") && parts.command("custom"),
      "Reset lost the project part or retained the built-in.",
    );
    await writeFile(
      resolve("docs/compaction/package.json"),
      JSON.stringify(
        {
          bundle: "Bun target bun, Parts entry, native furb external",
          disabled: true,
          enabled: true,
          existingCompactCall: true,
          reloadSingleCommand: true,
          resetRemovesBuiltin: true,
          projectPartPreserved: true,
        },
        null,
        2,
      ) + "\n",
    );
    console.log(
      "Packaged Parts: disabled, enabled, exact existing compact call, reload, reset, project part PASS",
    );
  } finally {
    parts.dispose();
  }
} finally {
  await rm(directory, { recursive: true, force: true });
}
