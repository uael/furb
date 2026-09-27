import type { Engine } from "../src/bridge.ts";

/** The python of each turn of a chain, as the life folds it. */
export const transcriptOf = async (life: Engine, chain: string): Promise<string[]> =>
  (await life.turns({ on: chain })).map(([, python]) => python);
