import { EditBuffer } from "@opentui/core";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";

const variant = process.argv[2];

async function cycle(file: string, terminate: boolean): Promise<Worker> {
  const worker = new Worker(new URL(file, import.meta.url).href, { type: "module" });
  await new Promise<void>((resolve) => worker.addEventListener("message", () => resolve(), { once: true }));
  if (terminate) worker.terminate();
  await Bun.sleep(200);
  return worker;
}

async function emit(label: string): Promise<void> {
  let delivered = 0;
  const buffer = EditBuffer.create("unicode");
  buffer.on("content-changed", () => {
    delivered++;
  });
  buffer.setText(label);
  await Bun.sleep(0);
  buffer.destroy();
  console.log(`${label} delivered ${delivered}`);
}

await emit("before");
for (let round = 0; round < 3; round++) {
  if (variant === "opentui-worker") await cycle("native-probe-worker.ts", true);
  if (variant === "plain-worker") await cycle("native-probe-plain.ts", true);
  if (variant === "opentui-worker-alive") await cycle("native-probe-worker.ts", false);
  if (variant === "session") await (await demoSession()).dispose();
  await emit(`after ${round}`);
}
await removeDemoDirectories();
process.exit(0);
