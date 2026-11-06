import { isDone } from "furb/life";
import { until } from "../../bind/typescript/test/until.ts";
import type { Session } from "../src/session.ts";

export const idle = (session: Session) =>
  until(session, () => !session.acts.some((act) => !isDone(act) && ["prompt", "rung"].includes(act.kind)));
