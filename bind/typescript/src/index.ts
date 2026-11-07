import { quotes, writing } from "../index.cjs";
import { readQuotesWith } from "./life/quotes.js";

export * from "../index.cjs";
export { Activity, type Answered, type LiveAct, type LiveRun } from "./activity.js";
export { DamagedJournal, type FileChange, FileChanges, type Repaired } from "./changes.js";
export { Console, type ConsoleOptions } from "./console.js";
export { type Call, call, driving, type Ear, type Saying, speaking } from "./ears.js";
export { Chain, Life, type LifeAct, type LifeOptions, open, Question, Thread } from "./framework.js";
export { type Act, type PauseCause, type RunState, WORK } from "./life/acts.js";
export { furbDirectory, saveFile } from "./project.js";
export {
  type Answer,
  boot,
  inspectRecord,
  type Retry,
  Session,
  type SessionOptions,
  type Stream,
} from "./session.js";
export type { Actor, Entry, Fact, Paragraph, Standing, Turn } from "./types.js";
export {
  actorParts,
  bound,
  display,
  efforts,
  isQuestion,
  opens,
  paragraphs,
  plain,
  questionKind,
  safeText,
  standingOf,
  unmarked,
  usageOf,
} from "./types.js";

// The model of a life reads each quote of a word as the crate reads it, through its door.
readQuotesWith({ quotes: (text, open) => quotes(text, open), writing });
