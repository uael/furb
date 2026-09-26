import type { Engine } from "../index.cjs";

/** What an ear says: the kind of the fact, the act it is about, and its words, and nothing of who says it, which the
 * bus fills in from whoever speaks. */
export type Saying = [kind: string, about: string, ...words: unknown[]];
/** A verb of the engine, which an ear calls while it hears, since nothing may enter the engine while it hears. */
export interface Call {
  verb: string;
  args?: unknown[];
  kwargs?: Record<string, unknown>;
}
/** An ear: a generator that hears every fact of the life. It yields a saying and hears back the fact the bus made of
 * it, yields a call and hears back what the verb gave, or has thrown in what it raised, and yields nothing to hear
 * the next fact. It hears nothing at its birth. */
export type Ear = Generator<Saying | Call | null | undefined, void, unknown>;

/** An ear of the outside that brings another to life as an ear of the engine at the birth of the life, and is over.
 * An ear of the engine hears every fact and every question, where an ear of the outside hears a question only when
 * no ear before it took it, so an ear that keeps every act is one. */
export function* driving(ear: Ear, name: string): Ear {
  yield { verb: "drive", args: [ear, name] };
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

/** What a host threw, as the engine reads a fault: a map that names its class, or a refusal of its message. */
export function fault(error: unknown): { is: string; args: unknown[] } {
  if (error && typeof error === "object" && "is" in error && "args" in error)
    return error as { is: string; args: unknown[] };
  return { is: "Refused", args: [error instanceof Error ? error.message : String(error)] };
}
