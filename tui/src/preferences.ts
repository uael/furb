import { mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { configDirectory, saveFile } from "@furb/engine";
import { defaultTheme, palettes, type ThemeName, themeLabels } from "./theme.ts";

export class Preferences {
  theme: ThemeName = defaultTheme;
  sidebar = true;
  sidebarWidth = 32;
  /** Whether a word starts folded in the feed, to its comments. */
  foldRungs = true;
  /** Whether what works moves: a spinner, the bar of a reply, the blink of the caret, a step that lands, and the check
   * of a closed thread. With no motion, a spinner is a still mark. */
  motion = true;
  notice = "";
  private saved = "";
  constructor(readonly path = join(configDirectory(), "ui.json")) {
    try {
      const saved = JSON.parse(readFileSync(path, "utf8")) as {
        theme?: string;
        sidebar?: boolean;
        sidebarWidth?: number;
        foldRungs?: boolean;
        motion?: boolean;
      } | null;
      if (saved?.theme && Object.hasOwn(palettes, saved.theme)) this.theme = saved.theme as ThemeName;
      if (typeof saved?.sidebar === "boolean") this.sidebar = saved.sidebar;
      if (typeof saved?.foldRungs === "boolean") this.foldRungs = saved.foldRungs;
      if (typeof saved?.motion === "boolean") this.motion = saved.motion;
      if (typeof saved?.sidebarWidth === "number")
        this.sidebarWidth = Math.max(26, Math.min(48, saved.sidebarWidth));
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT")
        this.notice = `Could not read preferences at ${path}. Using ${themeLabels[defaultTheme][0]} for this run. The file is unchanged. Click to dismiss.`;
    }
    this.saved = this.serialize(this.theme);
  }
  private serialize(theme: ThemeName): string {
    return JSON.stringify({
      theme,
      sidebar: this.sidebar,
      sidebarWidth: this.sidebarWidth,
      foldRungs: this.foldRungs,
      motion: this.motion,
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
