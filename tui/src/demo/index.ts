import { keyboard } from "./audit.ts";
import { ci } from "./ci.ts";
import { conductor } from "./conductor.ts";
import { guide } from "./guide.ts";
import { more } from "./more.ts";
import { rivals } from "./rivals.ts";
import type { Script } from "./script.ts";
import { search } from "./search.ts";
import { shortcut } from "./shortcut.ts";
import { side } from "./side.ts";
import { storage } from "./storage.ts";

/** Every script of the models of the demo life, which the worker of a demo session answers from. */
export const scripts: readonly Script[] = [
  ...conductor,
  ...storage,
  ...search,
  ...shortcut,
  ...guide,
  ...rivals,
  ...ci,
  ...more,
  ...side,
  ...keyboard,
];
