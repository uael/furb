import { call, type Engine } from "../index.cjs";

/** What an ear says: the kind of the fact, the act it is about, and its words, and nothing of who says it, which the
 * bus fills in from whoever speaks. */
export type Saying = [kind: string, about: string, ...words: unknown[]];
/** An ear: a generator that hears every fact of the life. It yields a saying and hears back the fact the bus made of
 * it, and yields nothing to hear the next fact. It hears nothing at its birth. While it hears, it calls a verb of
 * the engine with `call`, which gives what the verb gave or throws what it raised. */
export type Ear = Generator<Saying | null | undefined, void, unknown>;

/** An ear of the outside that brings another to life as an ear of the engine at the birth of the life, and is over.
 * An ear of the engine hears every fact and every question, where an ear of the outside hears a question only when
 * no ear before it took it, so an ear that keeps every act is one. */
// biome-ignore lint/correctness/useYield: an ear that brings another to life is over at its own birth.
export function* driving(ear: Ear, name: string): Ear {
  call("drive", [ear, name]);
}

/** What the work an ear began says later, said under the name of that ear, as the contract says the site of such work
 * is. */
export function speaking<T>(engine: Engine, name: string, action: () => T): T {
  const before = engine.site(name);
  try {
    return action();
  } finally {
    if (!engine.disposed) engine.site(before);
  }
}

/** Whether the work an ear began for an act has nothing more to say: the engine is gone, or the act is done. */
export function over(engine: Engine, id: string): boolean {
  return engine.disposed || engine.outcome(id).done;
}

/** Whether a value is a fault as the engine reads one: a map that names its class under `is`, and holds the list it
 * was made with under `args`. */
export function isFault(value: unknown): value is { is: string; args: unknown[] } {
  const held = (value ?? {}) as { is?: unknown; args?: unknown };
  return typeof held.is === "string" && Array.isArray(held.args);
}

/** What a host threw, as the engine reads a fault: a map that names its class, or a refusal of its message. */
export function fault(error: unknown): { is: string; args: unknown[] } {
  if (isFault(error)) return error;
  return { is: "Refused", args: [error instanceof Error ? error.message : String(error)] };
}
