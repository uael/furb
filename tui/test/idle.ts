import { until } from "../../bind/typescript/test/until.ts";
import type { Session } from "../src/session.ts";

/** The moment no prompt and no rung of the session is at work. */
export const idle = (session: Session) =>
  until(session, () => !session.acts.some((act) => !act.done && ["prompt", "rung"].includes(act.kind)));
