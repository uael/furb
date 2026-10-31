/** A function that runs a step which no request waits for, as a timer does. A fault of the step goes to `fault` as its
 * message, so the step cannot end the process that runs it. */
export function guarded(step: () => void, fault: (message: string) => void): () => void {
  return () => {
    try {
      step();
    } catch (error) {
      fault(error instanceof Error ? error.message : String(error));
    }
  };
}
