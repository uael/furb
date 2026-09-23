import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { defaultTheme, palettes, type ThemeName } from "./theme.ts";

export class Preferences {
  theme: ThemeName = defaultTheme;
  notice = "";
  constructor(
    readonly path = join(
      process.env.FURB_CONFIG_DIR ?? join(process.env.XDG_CONFIG_HOME ?? join(homedir(), ".config"), "furb"),
      "ui.json",
    ),
  ) {
    try {
      const saved = JSON.parse(readFileSync(path, "utf8")) as { theme?: string } | null;
      if (saved?.theme && Object.hasOwn(palettes, saved.theme)) this.theme = saved.theme as ThemeName;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT")
        this.notice = `Could not read preferences at ${path}. Using GitHub Dark. Click to dismiss.`;
    }
  }
  save(theme: ThemeName): void {
    if (theme === this.theme) return;
    mkdirSync(dirname(this.path), { recursive: true });
    writeFileSync(`${this.path}.tmp`, JSON.stringify({ theme }), { mode: 0o600 });
    renameSync(`${this.path}.tmp`, this.path);
    this.theme = theme;
  }
}
