import { copyFile, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { imageContent, imageReferences } from "@furb/engine";
import { display } from "@furb/engine/session";
import { Marked } from "marked";
import { conversation } from "./conversation.ts";
import type { Session } from "./session.ts";
import { palettes } from "./theme.ts";

const escaped = (value: string) =>
  value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
const markdown = new Marked({
  gfm: true,
  renderer: {
    html({ text }) {
      return escaped(text);
    },
    image({ text }) {
      return escaped(text);
    },
    link({ href, tokens }) {
      const text = this.parser.parseInline(tokens);
      return /^(https?:|mailto:|#)/i.test(href)
        ? `<a href="${escaped(href)}" rel="noreferrer">${text}</a>`
        : text;
    },
  },
});
const prose = (text: string) => markdown.parse(text, { async: false });

/** A standalone file with no scripts, network requests, or external assets. */
export function shareHtml(session: Session): string {
  const theme = palettes[session.theme];
  const sections: string[] = [];
  for (const item of conversation(session.turns, session.acts)) {
    if (item.type === "python")
      sections.push(`<section><h2>Python</h2><pre><code>${escaped(item.code)}</code></pre></section>`);
    else if (item.type === "prompt") {
      const message = String(item.act.words[1] ?? "");
      const references = imageReferences(message);
      const images = references
        .map((reference) => {
          const image = imageContent(session.host.imageDirectory, reference.uri);
          return `<img alt="Attached image" src="data:${image.mimeType};base64,${image.data}">`;
        })
        .join("");
      const text = references.reduce((rest, reference) => rest.replace(reference.text, ""), message);
      sections.push(
        `<section class="prompt"><h2>${session.isUserPrompt(item.act) ? "You" : "Observation"}</h2>${prose(text)}${images}</section>`,
      );
    } else if (item.type === "result")
      sections.push(`<section><h2>Result</h2>${prose(display(item.act.value))}</section>`);
    else if (item.type === "note")
      sections.push(
        `<details><summary>${escaped(item.label)} ${escaped(item.act?.id ?? item.detail)}</summary><pre>${escaped([item.detail, item.body].filter(Boolean).join("\n"))}</pre></details>`,
      );
  }
  return `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:"><title>${escaped(session.sessionName)} · furb</title><style>
body{color:${theme.text};background:${theme.background};font:16px/1.6 system-ui,sans-serif;max-width:900px;margin:48px auto;padding:0 24px}h1{font-size:28px}h2{font-size:14px;color:${theme.muted};font-weight:500}section{margin:32px 0}.prompt{background:${theme.panel};border-left:3px solid ${theme.accent};padding:12px 20px}p,pre{white-space:pre-wrap;overflow-wrap:anywhere}pre{font:14px/1.6 ui-monospace,monospace;background:${theme.panel};padding:16px}summary{cursor:pointer;color:${theme.muted}}details{margin:16px 0}footer{color:${theme.muted};font-size:13px;margin:48px 0}img{max-width:100%;height:auto}
</style><main><h1>${escaped(session.sessionName)}</h1><p>${escaped(session.label)}</p>${sections.join("\n")}<details><summary>Exact model transcript</summary>${session.turns.map(([, python]) => `<pre>${escaped(python)}</pre>`).join("\n")}</details></main><footer>Exported from furb. This file contains the selected chain's conversation and transcript.</footer></html>`;
}

export function shareMarkdown(session: Session): string {
  const lines = [`# ${session.sessionName}`, "", `Chain: ${session.label}`, ""];
  const fenced = (text: string, language: string) => {
    const fence = "`".repeat(Math.max(3, ...[...text.matchAll(/`+/g)].map((match) => match[0].length + 1)));
    return [`${fence}${language}`, text, fence];
  };
  for (const item of conversation(session.turns, session.acts)) {
    if (item.type === "python") lines.push("## Python", ...fenced(item.code, "python"), "");
    else if (item.type === "prompt") {
      const message = String(item.act.words[1] ?? "");
      lines.push(
        `## ${session.isUserPrompt(item.act) ? "You" : "Observation"}`,
        "",
        imageReferences(message).reduce(
          (text, reference) =>
            text.replace(reference.text, `[Image: ${reference.name}, included in conversation.html]`),
          message,
        ),
        "",
      );
    } else if (item.type === "result") lines.push("## Result", "", display(item.act.value), "");
    else if (item.type === "note")
      lines.push(
        `### ${item.label}`,
        ...fenced([item.detail, item.body].filter(Boolean).join("\n"), "text"),
        "",
      );
  }
  return lines.join("\n");
}

/** Called only after the user chooses to upload an unlisted GitHub gist. */
export async function publishShare(path: string, markdown: string, name: string): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), "furb-share-"));
  const file = join(directory, "conversation.md");
  const html = join(directory, "conversation.html");
  try {
    const binary = Bun.which("gh", { PATH: process.env.PATH });
    if (!binary) throw new Error("Install and sign in to gh before creating a share link.");
    await writeFile(file, markdown, { mode: 0o600 });
    await copyFile(path, html);
    const child = Bun.spawn([binary, "gist", "create", "--desc", name, file, html], {
      stdout: "pipe",
      stderr: "pipe",
    });
    const output = await new Response(child.stdout).text();
    if (await child.exited) throw new Error(await new Response(child.stderr).text());
    const url = output.trim();
    if (!/^https:\/\/gist\.github\.com\/[\w/-]+$/.test(url))
      throw new Error("GitHub did not return a share link.");
    return url;
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}
