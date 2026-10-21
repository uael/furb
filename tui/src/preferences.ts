import { mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { configDirectory, saveFile } from "@furb/engine";
import { defaultTheme, palettes, type ThemeName } from "./theme.ts";

export class Preferences {
  theme: ThemeName = defaultTheme;
  notice = "";
  private saved = "";
  constructor(readonly path = join(configDirectory(), "ui.json")) {
    try {
      const saved = JSON.parse(readFileSync(path, "utf8")) as {
        theme?: string;
      } | null;
      if (saved?.theme && Object.hasOwn(palettes, saved.theme)) this.theme = saved.theme as ThemeName;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT")
        this.notice = `Could not read preferences at ${path}. Using GitHub Dark for this run. The file is unchanged.`;
    }
    this.saved = this.serialize(this.theme);
  }
  private serialize(theme: ThemeName): string {
    return JSON.stringify({
      theme,
    });
  }
  save(theme = this.theme): void {
    const data = this.serialize(theme);
    if (data === this.saved) return;
    mkdirSync(dirname(this.path), { recursive: true });
    saveFile(this.path, data);
    this.theme = theme;
    this.saved = data;
    this.notice = "";
  }
}
