import { Daemon } from "../src/client.ts";

/** The life of a record ended in the daemon of the tests, whose lives end once their last window lets them go: a test
 * that reads or takes the record after its window let it go waits for that end, which writes its checkpoint and
 * frees its lease. */
export async function ended(record: string | undefined): Promise<void> {
  if (!record) throw new Error("The life keeps no record.");
  const asked = new Daemon();
  try {
    await asked.ended(record, 20000);
  } finally {
    asked.close();
  }
}
