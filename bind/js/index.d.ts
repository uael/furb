/**
 * furb for javascript: one life of the engine, in the sandbox, with a World you write in javascript.
 *
 * The crate runs the engine in monty and gives its surface to a host. This is that surface for a host of
 * javascript: `Life` is one life, a host writes a World and hands it over, and `Voice` is how the host says into
 * the life the work that finishes later.
 *
 * Nothing of the engine crosses. Every value that crosses is plain: null, a truth, a number, a text, an array and
 * an object are the javascript ones, `Tuple` is a tuple, which the engine reads apart from a list, `Shape` is a
 * shape of the engine such as a text or an exit, `Fault` is an exception by its name and what it was made with,
 * and `Show` is the mark of a show, which carries nothing.
 */

/** Value is everything that crosses between a life and its host. */
export type Value =
  | null
  | boolean
  | number
  | string
  | Value[]
  | { [name: string]: Value }
  | Tuple
  | Shape
  | Fault
  | Show

/** The name the World hears under, which is the name the engine takes it under. */
export const WORLD: string

/**
 * A tuple, which the engine reads apart from a list and javascript has no word for.
 *
 * Every fact is one of these, and so is the standing of a chain and the turn of a model. An array of javascript
 * is a list, and this is the other one.
 */
export class Tuple {
  constructor(items: Value[])
  /** What the tuple holds, in order. */
  get items(): Value[]
  /** How many values the tuple holds. */
  get length(): number
}

/**
 * A shape of the engine, by its name and its fields, which a text and an exit are.
 *
 * The fields keep the order the shape declares them in, and `shape.fields` reads them all at once, by name.
 */
export class Shape {
  constructor(name: string, fields: { [name: string]: Value })
  /** The name of the shape, as the engine holds it. */
  get name(): string
  /** The fields of the shape, by name, in the order the shape declares them. */
  get fields(): { [name: string]: Value }
}

/**
 * An exception of the engine, by its name and what it was made with.
 *
 * It is a value and not a throw: a fault the engine hands back is a thing a host reads, and only a fault of the
 * call a host made is thrown in the host.
 */
export class Fault {
  constructor(name: string, args?: Value[])
  /** The name of the exception. */
  get name(): string
  /** What the exception was made with. */
  get args(): Value[]
  /** Whether the engine refused the call, which is the one fault a word of a model makes on purpose. */
  refused(): boolean
}

/** The mark of a show, which carries nothing a host can read. */
export class Show {
  constructor()
}

/** One fact of the engine: its kind, the act it is about, who said it, and its words. */
export class Fact {
  constructor(kind: string, about: string, by?: string, words?: Value[])
  /** The kind of the fact, which is its first slot. */
  get kind(): string
  /** The act the fact is about. */
  get about(): string
  /** Who said the fact: a rung, the operator, the World or the Kernel. */
  get by(): string
  /** The words of the fact, which are everything after who said it. */
  get words(): Value[]
  /** Whether the fact is a question, which its name says: a question is about itself, under its kind. */
  get question(): boolean
  /** The chain a question is on, which is its first word, and nothing for a fact that is no question. */
  get on(): string | null
}

/**
 * A question of the engine, which a World answers a fact with when it must know something first.
 *
 * The boundary puts the question and hands the answer back through `World.answered`, so a World never calls into
 * a life that stands waiting for it.
 */
export interface Ask {
  /** The kind of the question, such as `cwd` or `merged`. */
  ask: string
  /** The chain the question is on. */
  on: string
  /** The words of the question. */
  words?: Value[]
}

/** Reply is what a World says of a fact: nothing, one fact, the facts to say in that order, or a question. */
export type Reply = null | undefined | Fact | Fact[] | Ask

/**
 * The interface to the disk, the machine, the actors and the record.
 *
 * The World hears every fact. It answers what a chain stands on, a reading of the clock, a number it draws, and a
 * read or a write of a path nobody of the engine serves; it starts what it is started to do, which is a command,
 * a wait and a prompt of the operator; it answers an ask with the turn of a model; and it keeps what the journal
 * says to keep.
 *
 * A World that does work which finishes later holds a `Voice` and says the facts of that work into it.
 */
export interface World {
  /** One fact, heard. What the World would say of it, or the question it must ask first. */
  hears(fact: Fact): Reply
  /** The answer to the question the World last asked, and what it says now that it holds it. */
  answered(got: Value): Reply
}

/**
 * What reads a word before it runs.
 *
 * The Kernel runs the word of a rung where the engine runs, so the one thing it cannot answer for itself is
 * whether the word may run at all: what a word is read against is the host's to decide.
 */
export interface Gate {
  /** What the gate finds against a word. Nothing at all means the word may run. */
  gate(word: string, ladder: string[], shape: string): string[]
}

/**
 * How a host says into a life when nothing asked it to.
 *
 * A Voice may be carried anywhere the host does its work, since what it says waits in order until the life is
 * ready to hear it. It is heard by one life: the life it is given to.
 */
export class Voice {
  constructor()
  /** One fact, said into the life. */
  fact(one: Fact): void
  /** One act, closed with a value, which answers the act that asked for it. */
  close(act: string, value: Value): void
  /** One chain, paused, which stops it until the operator wakes it. */
  pause(chain: string): void
  /** Whether anything said into this Voice is still waiting to be heard. */
  waiting(): boolean
}

/**
 * One life: the engine in the sandbox, and the outside it reaches.
 *
 * A host opens one with a World of its own and drives it by calling words on it, the way an operator calls a
 * verb. What the host started and has not finished it says through its `Voice`, and the life hears all of it in
 * the order it was said.
 */
export class Life {
  constructor(world: World, gate?: Gate | null, record?: string | null, voice?: Voice | null)
  /** The root chain of the life, which is the first act of any record. */
  get root(): string
  /** The World of this life, which is the object the host handed over. */
  get world(): World
  /** One word of the operator, run on a chain, and what it gave. */
  word(word: string): Value
  /** Everything the host has said into its Voice, done in the life, in the order it was said. */
  heard(): number
  /** Wait until the host says something, or until this long has passed, and say whether anything waits. */
  waits(seconds: number): boolean
  /** What an act came to, and nothing at all while it waits. */
  came(act: string): Value | null
}

/** The line a World writes for one entry it was told to keep. */
export function line(entry: Value): string
