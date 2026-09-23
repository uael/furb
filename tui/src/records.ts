/** A separate TUI worker keeps saved-session inspection away from input and rendering. */
export function inspectRecords(paths: string[]): Promise<Record<string, { held: number; error?: string }>> {
  if (!paths.length) return Promise.resolve({});
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./record-worker.ts", import.meta.url).href);
    worker.onmessage = ({ data }) => resolve(data);
    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(event.message));
    };
    worker.postMessage(paths);
  });
}
