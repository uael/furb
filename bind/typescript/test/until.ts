import type { EventEmitter } from "node:events";

/** The moment a condition holds, heard on the event that can change it: no test waits for time to pass. The
 * deadline comes before the thirty seconds that bunfig.toml gives a test, so the failure names what it waited for,
 * and what `seen` tells of the state it saw last. */
export function until(
  emitter: EventEmitter,
  ready: () => boolean,
  event = "change",
  seen?: () => unknown,
): Promise<void> {
  if (ready()) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const heard = () => {
      if (!ready()) return;
      clearTimeout(deadline);
      emitter.off(event, heard);
      resolve();
    };
    const deadline = setTimeout(() => {
      emitter.off(event, heard);
      const last = seen ? ` It saw ${JSON.stringify(seen())}.` : "";
      reject(new Error(`Waited 25 seconds for ${ready}.${last}`));
    }, 25000);
    emitter.on(event, heard);
  });
}
