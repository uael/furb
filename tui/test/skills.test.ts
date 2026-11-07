import { afterAll, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { demoSession } from "../src/demo.ts";
import { composing } from "./composing.ts";

const made: string[] = [];
afterAll(() => {
  for (const one of made) rmSync(one, { recursive: true, force: true });
});

/** A skill on the disk of the project: a folder with a SKILL.md file that opens with a frontmatter. */
function skill(directory: string, name: string, detail: string): void {
  const folder = join(directory, ".furb", "skills", name);
  mkdirSync(folder, { recursive: true });
  writeFileSync(join(folder, "SKILL.md"), `---\nname: ${name}\ndescription: ${detail}\n---\nSteps.\n`);
}

test("a skill added on the disk shows in the list of / commands after /skills, and its command loads the skill", async () => {
  const directory = mkdtempSync(join(tmpdir(), "furb-skills-"));
  made.push(directory);
  skill(directory, "brew", "Brew a pot of tea");
  const session = await demoSession({ cwd: directory, extensions: true });
  await composing(
    async ({ app, frame }) => {
      const typed = async (text: string, wanted: string, present: boolean) => {
        app.composer.setText("");
        app.composer.insertText(text);
        let shown = await frame();
        for (let tries = 0; tries < 400 && shown.includes(wanted) !== present; tries++) {
          await Bun.sleep(25);
          shown = await frame();
        }
        return shown;
      };
      expect(await typed("/br", "Brew a pot of tea (skill)", true)).toContain("/brew");
      skill(directory, "bake", "Bake a loaf of bread");
      expect(await typed("/ba", "Bake a loaf", false)).not.toContain("/bake");
      app.composer.setText("/reload-skills");
      await app.submit();
      expect(await typed("/ba", "Bake a loaf of bread (skill)", true)).toContain("/bake");
      expect(session.notice).toContain("Skills read again");
      app.composer.setText("/bake");
      await app.submit();
      for (
        let tries = 0;
        tries < 400 && !session.acts.some((act) => act.kind === "skill" && act.words[0] === "bake");
        tries++
      )
        await Bun.sleep(25);
      expect(session.acts.some((act) => act.kind === "skill" && act.words[0] === "bake")).toBe(true);
    },
    { width: 132, height: 50 },
    session,
  );
});
