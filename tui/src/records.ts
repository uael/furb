/** A separate TUI worker keeps saved-session inspection away from input and rendering. An abort ends the worker and
 * the replays it has not made. */
export function inspectRecords(
  paths: string[],
  signal: AbortSignal,
): Promise<Record<string, { pending: number; error?: string }>> {
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
    worker.postMessage(paths);
  });
}
