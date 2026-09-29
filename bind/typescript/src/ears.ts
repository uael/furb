import type { Engine } from "../index.cjs";

/** What an ear says: the kind of the fact, the act it is about, and its words, and nothing of who says it, which the
 * bus fills in from whoever speaks. */
export type Saying = [kind: string, about: string, ...words: unknown[]];
/** A verb of the engine as an ear yields it: its name and its words. */
export type Call = { call: string; args: unknown[]; kwargs: Record<string, unknown> };
/** An ear: a generator that hears every fact of the life. It yields a saying and hears back the fact the bus made of
 * it, and yields nothing to hear the next fact. It hears nothing at its birth. While it hears, it calls a verb of
 * the engine by yielding `call(verb, args, kwargs)`, and hears back what the verb gave, or has what it raised thrown
 * where it yielded. */
export type Ear = Generator<Saying | Call | null | undefined, void, unknown>;

/** A verb of the engine, called by its name with its words by the ear that yields it. */
export function call(verb: string, args: unknown[] = [], kwargs: Record<string, unknown> = {}): Call {
  return { call: verb, args, kwargs };
}

/** An ear of the outside that brings another to life as an ear of the engine at the birth of the life, and is over.
 * An ear of the engine hears every fact and every question, where an ear of the outside hears a question only when
 * no ear before it took it, so an ear that keeps every act is one. */
export function* driving(ear: Ear, name: string): Ear {
  yield call("drive", [ear, name]);
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
