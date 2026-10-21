import { python } from "@codemirror/lang-python";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { RangeSetBuilder } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, WidgetType } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import CodeMirror from "@uiw/react-codemirror";
import { useMemo } from "react";
import type { ActMark, Step } from "../data.ts";

/** The colors of code: quiet, so the values of the acts beside it read first. */
const highlight = HighlightStyle.define([
  { tag: [tags.keyword, tags.controlKeyword, tags.operatorKeyword], color: "var(--accent)" },
  {
    tag: [tags.string, tags.special(tags.string)],
    color: "color-mix(in srgb, var(--done) 80%, var(--text))",
  },
  { tag: [tags.number, tags.bool, tags.null], color: "color-mix(in srgb, var(--asks) 85%, var(--text))" },
  { tag: [tags.function(tags.variableName)], color: "var(--text)", fontWeight: "500" },
  { tag: tags.comment, color: "var(--faint)", fontStyle: "italic" },
  { tag: [tags.variableName, tags.propertyName], color: "var(--text)" },
  { tag: [tags.punctuation, tags.operator, tags.paren], color: "var(--muted)" },
]);

const marks: Record<string, string> = {
  asks: "◆",
  failed: "✕",
  done: "✓",
  runs: "●",
  waits: "○",
  paused: "‖",
  rest: "○",
};

/** The value of one act, at the end of the line that made it. */
class Value extends WidgetType {
  constructor(readonly act: ActMark) {
    super();
  }
  override eq(other: Value) {
    return (
      other.act.id === this.act.id &&
      other.act.state === this.act.state &&
      other.act.summary === this.act.summary
    );
  }
  toDOM() {
    const chip = document.createElement("span");
    chip.className = `furb-value furb-value-${this.act.state}`;
    chip.dataset.act = this.act.id;
    chip.title = `${this.act.id}: ${this.act.summary}`;
    const mark = document.createElement("span");
    mark.className = "furb-value-mark";
    mark.textContent = marks[this.act.state] ?? "";
    const words = document.createElement("span");
    words.textContent = this.act.summary;
    chip.append(mark, words);
    return chip;
  }
  override ignoreEvent() {
    return false;
  }
}

/** The head of a step, between two words of a ladder. */
class Head extends WidgetType {
  constructor(
    readonly step: Step,
    readonly n: number,
  ) {
    super();
  }
  override eq(other: Head) {
    return other.step.id === this.step.id && other.step.said === this.step.said && other.n === this.n;
  }
  toDOM() {
    const head = document.createElement("div");
    head.className = `furb-step furb-step-${this.step.state}`;
    head.textContent = `Step ${this.n}  ${this.step.said}`;
    return head;
  }
}

/** Why a word failed, under it. */
class Why extends WidgetType {
  constructor(readonly text: string) {
    super();
  }
  override eq(other: Why) {
    return other.text === this.text;
  }
  toDOM() {
    const why = document.createElement("div");
    why.className = "furb-why";
    why.textContent = this.text;
    return why;
  }
}

const look = EditorView.theme({
  ".furb-value": {
    display: "inline-flex",
    alignItems: "center",
    gap: "5px",
    marginLeft: "14px",
    padding: "0 8px",
    borderRadius: "999px",
    fontFamily: "var(--font-sans)",
    fontSize: "11.5px",
    lineHeight: "19px",
    color: "var(--muted)",
    background: "color-mix(in srgb, var(--text) 6%, transparent)",
    cursor: "pointer",
    verticalAlign: "1px",
    maxWidth: "340px",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  ".furb-value:hover": { color: "var(--text)" },
  ".furb-value-mark": { fontSize: "9px" },
  ".furb-value-asks": {
    color: "var(--asks)",
    background: "color-mix(in srgb, var(--asks) 14%, transparent)",
  },
  ".furb-value-failed": {
    color: "var(--failed)",
    background: "color-mix(in srgb, var(--failed) 12%, transparent)",
  },
  ".furb-value-done .furb-value-mark": { color: "var(--done)" },
  ".furb-value-runs .furb-value-mark": { color: "var(--accent)" },
  ".furb-step": {
    fontFamily: "var(--font-sans)",
    fontSize: "11.5px",
    color: "var(--faint)",
    padding: "10px 0 4px",
    letterSpacing: "0.01em",
  },
  ".furb-why": {
    fontFamily: "var(--font-sans)",
    fontSize: "12px",
    color: "var(--failed)",
    padding: "2px 0 6px",
    whiteSpace: "normal",
  },
});

/** A ladder as code: the words of its steps as one program, the line that runs, and the value of each act beside the
 * line that made it. `heads` puts the head of each step above its word. */
export function Code({
  steps,
  heads = true,
  numbers = true,
  wrap = false,
  onPick,
}: {
  steps: Step[];
  heads?: boolean;
  numbers?: boolean;
  wrap?: boolean;
  onPick?: (act: string) => void;
}) {
  const { doc, extensions } = useMemo(() => {
    const texts: string[] = [];
    for (const step of steps) for (const line of step.lines) texts.push(line.text);
    const doc = texts.join("\n");
    const decorations = EditorView.decorations.compute([], (state) => {
      const builder = new RangeSetBuilder<Decoration>();
      let n = 1;
      steps.forEach((step, at) => {
        const first = n;
        for (const line of step.lines) {
          const range = state.doc.line(n);
          if (n === first && heads && (at > 0 || steps.length > 1)) {
            builder.add(
              range.from,
              range.from,
              Decoration.widget({ widget: new Head(step, at + 1), block: true, side: -1 }),
            );
          }
          const classes = [
            line.here ? "furb-here" : "",
            line.here && line.acts.some((one) => one.state === "asks") ? "furb-asks" : "",
            step.state === "failed" ? "furb-failed" : "",
          ]
            .filter(Boolean)
            .join(" ");
          if (classes) builder.add(range.from, range.from, Decoration.line({ class: classes }));
          for (const act of line.acts) {
            builder.add(range.to, range.to, Decoration.widget({ widget: new Value(act), side: 1 }));
          }
          if (n === first + step.lines.length - 1) {
            const why = step.findings ?? step.raised;
            if (why)
              builder.add(
                range.to,
                range.to,
                Decoration.widget({ widget: new Why(why), block: true, side: 1 }),
              );
          }
          n += 1;
        }
      });
      return builder.finish() as DecorationSet;
    });
    const clicks = EditorView.domEventHandlers({
      mousedown: (event) => {
        const chip = (event.target as HTMLElement).closest<HTMLElement>(".furb-value");
        if (chip?.dataset.act && onPick) {
          onPick(chip.dataset.act);
          return true;
        }
        return false;
      },
    });
    return {
      doc,
      extensions: [
        python(),
        syntaxHighlighting(highlight),
        look,
        decorations,
        clicks,
        EditorView.editable.of(false),
        ...(wrap ? [EditorView.lineWrapping] : []),
      ],
    };
  }, [steps, heads, wrap, onPick]);
  return (
    <CodeMirror
      value={doc}
      editable={false}
      readOnly
      theme="none"
      extensions={extensions}
      onCreateEditor={(view) => {
        // The gutter measures the lines once; it measures them again when the mono face has loaded.
        void document.fonts.ready.then(() => view.requestMeasure());
      }}
      basicSetup={{
        lineNumbers: numbers,
        foldGutter: false,
        highlightActiveLine: false,
        highlightActiveLineGutter: false,
        highlightSelectionMatches: false,
        bracketMatching: false,
        closeBrackets: false,
        autocompletion: false,
        searchKeymap: false,
      }}
    />
  );
}
