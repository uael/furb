/** What the worker tells of a record: the name that a plain scan of the record found, and what its replay told when it
 * replayed it. */
export interface RecordState {
  /** The state that the engine of the crate gives the record: runs when it holds work that the next open resumes, and
   * saved when it holds none or was only scanned. */
  state: "runs" | "saved";
  error?: string;
  reading?: boolean;
  /** The name of the session that the fold of the crate gives the record: the one that the operator said last, else the
   * one that the extension naming gave. */
  name?: string;
  /** The last name that the operator gave the session, which the crate read in the record. */
  said?: string;
  /** Whether the record was replayed, or only scanned. */
  replayed: boolean;
}

/** A separate TUI worker keeps saved-session inspection away from input and rendering: it scans each record for its
 * name and replays the records of replay. An abort ends the worker and the reads it has not made. */
export function inspectRecords(
  paths: string[],
  signal: AbortSignal,
  replay: string[] = paths,
): Promise<Record<string, RecordState>> {
  if (!paths.length) return Promise.resolve({});
  if (signal.aborted) return Promise.reject(signal.reason);
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./record-worker.ts", import.meta.url).href);
    const stop = () => {
      worker.terminate();
      reject(signal.reason);
    };
    signal.addEventListener("abort", stop, { once: true });
    worker.onmessage = ({ data }) => {
      signal.removeEventListener("abort", stop);
      resolve(data);
    };
    worker.onerror = (event) => {
      signal.removeEventListener("abort", stop);
      worker.terminate();
      reject(new Error(event.message));
    };
    worker.postMessage({ paths, replay });
  });
}
