import { copyFile, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { imageContent } from "@furb/engine";
import { display, isTag } from "@furb/engine/world";
import { Marked } from "marked";
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
  for (const [role, parts] of session.turns)
    for (const part of parts) {
      if (typeof part === "string") {
        sections.push(
          `<section><h2>${role === "assistant" ? "Python" : "Message"}</h2><pre><code>${escaped(part)}</code></pre></section>`,
        );
        continue;
      }
      if (!isTag(part)) continue;
      const [kind, attrs, body] = part;
      const fields = Object.fromEntries(attrs);
      const act = session.acts.find((act) => act.id === (fields.id ?? fields.over));
      if (kind === "opened" && act?.kind === "prompt") {
        const message = String(fields.message ?? "");
        const images = [...message.matchAll(/furb-image:\/\/[a-f0-9]{64}\.(?:png|jpg|gif|webp)/g)]
          .map((match) => {
            const image = imageContent(session.world.imageDirectory, match[0]);
            return `<img alt="Attached image" src="data:${image.mimeType};base64,${image.data}">`;
          })
          .join("");
        sections.push(
          `<section class="prompt"><h2>${session.isUserPrompt(act) ? "You" : "Observation"}</h2>${prose(message.replace(/!\[[^\]]*\]\(furb-image:\/\/[^)]+\)/g, ""))}${images}</section>`,
        );
      } else if (kind === "closed" && act?.kind === "prompt")
        sections.push(`<section><h2>Result</h2>${prose(display(act.value ?? body))}</section>`);
      else if (!["opened", "closed", "ledger"].includes(kind))
        sections.push(
          `<details><summary>${escaped(kind)} ${escaped(String(fields.path ?? fields.id ?? ""))}</summary><pre>${escaped(display(body))}</pre></details>`,
        );
    }
  return `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:"><title>${escaped(session.sessionName)} · furb</title><style>
body{color:${theme.text};background:${theme.background};font:16px/1.6 system-ui,sans-serif;max-width:900px;margin:48px auto;padding:0 24px}h1{font-size:28px}h2{font-size:14px;color:${theme.muted};font-weight:500}section{margin:32px 0}.prompt{background:${theme.panel};border-left:3px solid ${theme.accent};padding:12px 20px}p,pre{white-space:pre-wrap;overflow-wrap:anywhere}pre{font:14px/1.6 ui-monospace,monospace;background:${theme.panel};padding:16px}summary{cursor:pointer;color:${theme.muted}}details{margin:16px 0}footer{color:${theme.muted};font-size:13px;margin:48px 0}img{max-width:100%;height:auto}
</style><main><h1>${escaped(session.sessionName)}</h1><p>${escaped(session.label)}</p>${sections.join("\n")}<details><summary>Exact model transcript</summary>${session.rendered.map((text) => `<pre>${escaped(text)}</pre>`).join("\n")}</details></main><footer>Exported from furb. This file contains the selected chain's conversation and transcript.</footer></html>`;
}

export function shareMarkdown(session: Session): string {
  const lines = [`# ${session.sessionName}`, "", `Chain: ${session.label}`, ""];
  for (const [role, parts] of session.turns)
    for (const part of parts) {
      if (typeof part === "string") {
        const fence = "`".repeat(
          Math.max(3, ...[...part.matchAll(/`+/g)].map((match) => match[0].length + 1)),
        );
        lines.push(`## ${role === "assistant" ? "Python" : "Message"}`, `${fence}python`, part, fence, "");
      } else if (isTag(part)) {
        const fields = Object.fromEntries(part[1]);
        const act = session.acts.find((act) => act.id === (fields.id ?? fields.over));
        if (part[0] === "opened" && act?.kind === "prompt")
          lines.push(
            `## ${session.isUserPrompt(act) ? "You" : "Observation"}`,
            "",
            String(fields.message ?? "").replace(
              /!\[([^\]]*)\]\(furb-image:\/\/[^)]+\)/g,
              "[Image: $1, included in conversation.html]",
            ),
            "",
          );
        if (part[0] === "closed" && act?.kind === "prompt")
          lines.push("## Result", "", display(act.value ?? part[2]), "");
        else if (!["opened", "closed", "ledger"].includes(part[0])) {
          const body = display(part[2]);
          const fence = "`".repeat(
            Math.max(3, ...[...body.matchAll(/`+/g)].map((match) => match[0].length + 1)),
          );
          lines.push(`### ${part[0]}`, `${fence}text`, body, fence, "");
        }
      }
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
