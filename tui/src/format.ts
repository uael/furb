import type { Retry } from "furb";
import { clock } from "./clock.ts";

/** What the footer says of a retry of the call of a model: the seconds left of its wait, its attempt, why the
 * attempt before it failed, and the commands that end the wait. A wait for the end of a usage limit or for a sign in
 * has no attempt and no countdown, since it lasts until a date or a sign in, so the footer says why alone. When more
 * than one call waits, it counts them. */
export function retryLine(retry: Retry, waiting = 1, now = clock()): string {
  const chains = waiting > 1 ? `, ${waiting} chains wait` : "";
  if (/^(usage limit|signed out)/.test(retry.why)) return `${retry.why}${chains}: /retry or /cancel`;
  const seconds = Math.max(0, Math.ceil((retry.until - now) / 1000));
  return `Retrying in ${seconds} s, attempt ${retry.attempt} of ${retry.most}, ${retry.why}${chains}: /retry or /cancel`;
}

/** A failure in plain words: a file that holds data that cannot be read says so, in place of the words of the parser
 * of JSON, and any other failure says its own message. */
export function plainFailure(message: string): string {
  return /JSON Parse error|is not valid JSON|JSON\.parse|Unexpected (token|end of JSON|character|identifier)/.test(
    message,
  )
    ? "A file of this session holds data that cannot be read. Refresh the view to read it again."
    : message;
}

/** A count of tokens, short: 1234 is 1.2k. */
export function count(value: number): string {
  return value >= 1_000_000
    ? `${Number((value / 1_000_000).toFixed(1))}M`
    : value >= 1000
      ? `${Number((value / 1000).toFixed(1))}k`
      : String(value);
}

const money = (digits: number) =>
  new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
const cents = money(2),
  fractions = money(4);
/** An amount of dollars: to four places under a dollar, where each fraction of a cent counts, and to the cent above. */
export function dollars(value: number): string {
  return (value !== 0 && Math.abs(value) < 1 ? fractions : cents).format(value);
}

/** The lines that a patch adds and removes, with no header of a file or a hunk. */
export function lineCounts(patch: string): { added: number; removed: number } {
  let added = 0,
    removed = 0;
  for (const line of patch.split("\n"))
    if (line.startsWith("+") && !line.startsWith("+++")) added++;
    else if (line.startsWith("-") && !line.startsWith("---")) removed++;
  return { added, removed };
}

/** A size in bytes, in KiB. */
export function kibibytes(bytes: number): string {
  return `${(bytes / 1024).toFixed(1)} KiB`;
}

/** A share of the window of a model. */
export function share(value: number): string {
  return `${Number((value * 100).toFixed(1))}% context`;
}

/** The graphemes of a text, which the terminal draws and the composer deletes one at a time. */
export const graphemes = new Intl.Segmenter();
/** A line cut to a width of terminal cells, with an ellipsis where it was cut: at its end, or at its start to keep
 * the end of a path. */
export function clip(line: string, width: number, keep: "start" | "end" = "start"): string {
  if (Bun.stringWidth(line) <= width) return line;
  const parts = [...graphemes.segment(line)].map(({ segment }) => segment);
  if (keep === "end") parts.reverse();
  let result = "";
  for (const part of parts) {
    if (Bun.stringWidth(result + part) >= width - 1) break;
    result = keep === "end" ? part + result : result + part;
  }
  return keep === "end" ? `…${result}` : `${result}…`;
}

/** A span of time, short: 42s, 3m 05s, 1h 02m. */
export function elapsed(milliseconds: number): string {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${String(seconds % 60).padStart(2, "0")}s`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

const RECORD = /^(\d{4})-(\d\d)-(\d\d)T(\d\d)-(\d\d)-(\d\d)\.\d+Z-[0-9a-f]+$/;

/** What a person reads for a record from its name, which starts with the time that it was made: "Oct 28, 07:45", in
 * the time of the machine. A name of another form stays as it is. */
export function recordLabel(name: string): string {
  const parts = RECORD.exec(name)?.slice(1).map(Number);
  if (!parts) return name;
  const [year = 0, month = 1, day = 1, hour = 0, minute = 0, second = 0] = parts;
  const when = new Date(Date.UTC(year, month - 1, day, hour, minute, second));
  const date = when.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  return `${date}, ${String(when.getHours()).padStart(2, "0")}:${String(when.getMinutes()).padStart(2, "0")}`;
}

/** How long ago a time was, in the largest unit that says it, and the date past a week. */
export function ago(time: number, now = clock()): string {
  const minutes = Math.floor((now - time) / 60_000);
  const say = (value: number, unit: string) => `${value} ${unit}${value === 1 ? "" : "s"} ago`;
  if (minutes < 1) return "just now";
  if (minutes < 60) return say(minutes, "minute");
  if (minutes < 60 * 24) return say(Math.floor(minutes / 60), "hour");
  if (minutes < 60 * 24 * 7) return say(Math.floor(minutes / 60 / 24), "day");
  return new Date(time).toLocaleDateString();
}

/** The provider and the id of a model by its name, provider:id. A name with no provider is an id alone. */
export function modelName(name: string): { provider: string; id: string } {
  const at = name.indexOf(":");
  return at < 0 ? { provider: "", id: name } : { provider: name.slice(0, at), id: name.slice(at + 1) };
}

/** A text that HTML and XML read as the text itself, with no mark of either. */
export const escaped = (value: string) =>
  value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
