/** Wraps a step that no request waits for, such as a timer. A fault of the step goes to `fault` as its message.
 * The step cannot end the process that runs it. */
export function guarded(step: () => void, fault: (message: string) => void): () => void {
  return () => {
    try {
      step();
    } catch (error) {
      fault(error instanceof Error ? error.message : String(error));
    }
  };
}
