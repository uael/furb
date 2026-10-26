import { diffChars } from "diff";
import { formatInput, formatOutput, type Shaped, statements } from "furb/life";

/** Resolve the installed executable once per environment change, not once per source word. */
function resolver(): () => string | null {
  let environment: string | undefined;
  let command: string | null = null;
  return () => {
    const identity = JSON.stringify([process.env.FURB_RUFF, process.env.PATH]);
    if (identity !== environment) {
      environment = identity;
      command = process.env.FURB_RUFF === undefined ? Bun.which("ruff") : process.env.FURB_RUFF || null;
    }
    return command;
  };
}

type Layout = [number, number, (string | [number, number])[]][];
type Job = { word: string; priority: number; owners: Set<string> };

/** A small layout references unchanged spans of the ORIGINAL source, rather than keeping another full word per step.
 * A complex rewrite exceeds the metadata budget and stays raw, consistently, for that step. */
function layout(word: string, shaped?: Shaped): Layout | undefined {
  if (!shaped?.size) return undefined;
  const source = word.split("\n");
  const out: Layout = [];
  let budget = 4096;
  const deadline = performance.now() + 8;
  for (const [from, part] of shaped) {
    const original = source.slice(from, part.to + 1).join("\n");
    const pieces: (string | [number, number])[] = [];
    let offset = 0;
    if (performance.now() >= deadline) return undefined;
    const changes = diffChars(original, part.lines.join("\n"), {
      timeout: Math.max(1, deadline - performance.now()),
      maxEditLength: 256,
    });
    if (!changes) return undefined;
    for (const change of changes) {
      if (change.added) {
        budget -= change.value.length + 16;
        pieces.push(change.value);
      } else {
        if (!change.removed) {
          budget -= 16;
          pieces.push([offset, offset + change.value.length]);
        }
        offset += change.value.length;
      }
      if (budget < 0) return undefined;
    }
    out.push([from, part.to, pieces]);
  }
  return out;
}

function restore(word: string, layout?: Layout): Shaped | undefined {
  if (!layout) return undefined;
  const source = word.split("\n");
  return new Map(
    layout.map(([from, to, pieces]) => {
      const original = source.slice(from, to + 1).join("\n");
      return [
        from,
        {
          to,
          lines: pieces
            .map((one) => (typeof one === "string" ? one : original.slice(one[0], one[1])))
            .join("")
            .split("\n"),
        },
      ];
    }),
  );
}

/** Per-window Python display formatting. No formatted text is sent to the engine. */
export class Formatter {
  private readonly kept = new Map<string, Layout | undefined>();
  private readonly waiting: Job[] = [];
  private readonly pending = new Map<string, Job>();
  private readonly steps = new Map<string, { hash: number | bigint; generation: number; layout?: Layout }>();
  private running?: Promise<void>;
  private process?: ReturnType<typeof Bun.spawn>;
  private command: string | null = null;
  private generation = 0;
  private disposed = false;
  private readonly offered = new Set<string>();
  version = 0;

  constructor(
    private readonly changed: () => void,
    private readonly configured: () => string | null = resolver(),
  ) {}

  private refresh(): void {
    const command = this.configured();
    if (command === this.command) return;
    this.command = command;
    this.generation++;
    this.process?.kill();
    this.waiting.length = 0;
    this.pending.clear();
    this.kept.clear();
    this.steps.clear();
    this.version++;
  }

  /** Refresh executable identity once per render, including when all program cards are cached. */
  begin(): void {
    if (this.disposed) return;
    this.offered.clear();
    this.refresh();
  }

  shape = (word: string, owner?: { id: string }, priority = 0): Shaped | undefined => {
    if (this.disposed) return undefined;
    this.refresh();
    const hash = Bun.hash(word);
    const saved = owner && this.steps.get(owner.id);
    if (saved && saved.hash === hash && saved.generation === this.generation)
      return restore(word, saved.layout);
    if (this.kept.has(word)) {
      const layout = this.kept.get(word);
      if (owner) this.steps.set(owner.id, { hash, generation: this.generation, layout });
      return restore(word, layout);
    }
    if (owner && this.pending.has(word)) {
      const job = this.pending.get(word);
      job?.owners.add(owner.id);
      if (job) job.priority = Math.max(job.priority, priority);
      return undefined;
    }
    if (!this.command || word.length > 131072) return undefined;
    // Owner-less requests are useful for pure tests. App steps instead retain their tiny chosen layout, including
    // raw fallback, by the stable step ID: cache eviction never renumbers an already formatted program.
    if (!owner) {
      if (!this.offered.has(word) && this.offered.size >= 256) return undefined;
      this.offered.add(word);
      if (this.pending.has(word)) return undefined;
    }
    if (this.waiting.length >= 64) {
      const lowest = this.waiting.reduce((a, b) => (a.priority <= b.priority ? a : b));
      if (priority <= lowest.priority) return undefined;
      this.waiting.splice(this.waiting.indexOf(lowest), 1);
      this.pending.delete(lowest.word);
    }
    const job = { word, priority, owners: new Set(owner ? [owner.id] : []) };
    this.waiting.push(job);
    this.pending.set(word, job);
    this.start();
    return undefined;
  };

  private start(): void {
    if (this.running || this.disposed || !this.waiting.length) return;
    this.running = this.drain().finally(() => {
      this.running = undefined;
      // A completion callback can enqueue after drain's last queue check but before this promise settles.
      this.start();
    });
  }

  private async drain(): Promise<void> {
    while (!this.disposed && this.waiting.length) {
      this.waiting.sort((a, b) => b.priority - a.priority);
      const job = this.waiting.shift();
      const command = this.command;
      if (!job || !command) continue;
      const { word } = job;
      const generation = this.generation;
      const made = await this.format(word, command).catch(() => undefined);
      if (this.disposed || generation !== this.generation) continue;
      this.pending.delete(word);
      const chosen = layout(word, made);
      this.kept.set(word, chosen);
      for (const owner of job.owners)
        this.steps.set(owner, { hash: Bun.hash(word), generation, layout: chosen });
      while (this.kept.size > 256) {
        const oldest = this.kept.keys().next().value;
        if (oldest === undefined) break;
        this.kept.delete(oldest);
      }
      // Even an unchanged or failed job frees a queue slot. Notify once so words that met a full queue get a turn.
      this.version++;
      this.changed();
    }
  }

  async idle(): Promise<void> {
    while (this.running) await this.running;
  }

  dispose(): void {
    this.disposed = true;
    this.generation++;
    this.waiting.length = 0;
    this.pending.clear();
    this.kept.clear();
    this.steps.clear();
    this.process?.kill();
  }

  private async format(word: string, command: string): Promise<Shaped | undefined> {
    const parts = statements(word);
    const input = formatInput(word, parts);
    if (!input.trim()) return undefined;
    const run = Bun.spawn(
      [command, "format", "--isolated", "--line-length", "100", "--stdin-filename", "word.py", "-"],
      { stdin: new Blob([input]), stdout: "pipe", stderr: "pipe" },
    );
    this.process = run;
    const timer = setTimeout(() => run.kill(), 3000);
    try {
      const [out, , code] = await Promise.all([
        new Response(run.stdout).text(),
        new Response(run.stderr).text(),
        run.exited,
      ]);
      if (code !== 0) return undefined;
      const made = formatOutput(word, parts, out);
      return (
        made &&
        new Map(
          [...made].flatMap(([index, lines]) => {
            const part = parts[index];
            return part ? [[part.from, { to: part.to, lines }] as const] : [];
          }),
        )
      );
    } finally {
      clearTimeout(timer);
      if (this.process === run) this.process = undefined;
    }
  }
}
