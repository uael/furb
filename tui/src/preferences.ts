import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { defaultTheme, palettes, type ThemeName } from "./theme.ts";

export class Preferences {
  theme: ThemeName = defaultTheme;
  sidebar = true;
  sidebarWidth = 28;
  autoCollapseRungs = false;
  notice = "";
  private saved = "";
  constructor(
    readonly path = join(
      process.env.FURB_CONFIG_DIR ??
        join(
          process.env.XDG_CONFIG_HOME ??
            (process.platform === "win32" ? process.env.APPDATA : undefined) ??
            join(homedir(), ".config"),
          "furb",
        ),
      "ui.json",
    ),
  ) {
    try {
      const saved = JSON.parse(readFileSync(path, "utf8")) as {
        theme?: string;
        sidebar?: boolean;
        sidebarWidth?: number;
        autoCollapseRungs?: boolean;
      } | null;
      if (saved?.theme && Object.hasOwn(palettes, saved.theme)) this.theme = saved.theme as ThemeName;
      if (typeof saved?.sidebar === "boolean") this.sidebar = saved.sidebar;
      if (typeof saved?.autoCollapseRungs === "boolean") this.autoCollapseRungs = saved.autoCollapseRungs;
      if (typeof saved?.sidebarWidth === "number")
        this.sidebarWidth = Math.max(22, Math.min(42, saved.sidebarWidth));
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT")
        this.notice = `Could not read preferences at ${path}. Using GitHub Dark for this run. The file is unchanged. Click to dismiss.`;
    }
    this.saved = this.serialize(this.theme);
  }
  private serialize(theme: ThemeName): string {
    return JSON.stringify({
      theme,
      sidebar: this.sidebar,
      sidebarWidth: this.sidebarWidth,
      autoCollapseRungs: this.autoCollapseRungs,
    });
  }
  save(theme = this.theme): void {
    const data = this.serialize(theme);
    if (data === this.saved) return;
    mkdirSync(dirname(this.path), { recursive: true });
    writeFileSync(`${this.path}.tmp`, data, { mode: 0o600 });
    renameSync(`${this.path}.tmp`, this.path);
    this.theme = theme;
    this.saved = data;
    this.notice = "";
  }
}
