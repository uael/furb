import type { EventEmitter } from "node:events";

/** The moment a condition holds, heard on the event that can change it: no test waits for time to pass. The
 * deadline fails the test loudly with what it waited for, where a silent wait would hang until the test timeout. */
export function until(emitter: EventEmitter, ready: () => boolean, event = "change"): Promise<void> {
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
      reject(new Error(`Waited 10 seconds for ${ready}.`));
    }, 10000);
    emitter.on(event, heard);
  });
}
