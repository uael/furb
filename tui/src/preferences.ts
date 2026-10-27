import { mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { ThemeMode } from "@opentui/core";
import { configDirectory, saveFile } from "furb";
import { isLifeView, type LifeView } from "./overview.ts";
import { isThemeChoice, type ThemeChoice } from "./theme.ts";

/** The shapes that the sidebar can take: an outline of the workspaces with the threads under the session shown, cards
 * of the threads under a switcher of the workspaces, and the outline with the questions of every session on top. */
export const railStyles = ["outline", "cards", "inbox"] as const;
export type RailStyle = (typeof railStyles)[number];
export const isRailStyle = (value: unknown): value is RailStyle =>
  typeof value === "string" && (railStyles as readonly string[]).includes(value);

export class Preferences {
  /** The theme that every session shows: one by its name, or the one that follows the terminal. */
  theme: ThemeChoice = "terminal";
  sidebar = true;
  sidebarWidth = 56;
  /** The shape of the sidebar. */
  sidebarStyle: RailStyle = "cards";
  /** Whether a thread that is over, and is not the last of its chain, starts folded in the feed. */
  foldThreads = true;
  /** Whether the welcome showed the check of the first run, which it shows once, and again while no model answers. */
  setupSeen = false;
  /** The view of the life that the TUI shows: main, the loom, or the graph. */
  view: LifeView = "main";
  /** Whether the view serves a screen reader: one column of text, with no box, no spinner, and nothing that moves. */
  screenReader = false;
  /** Whether the view holds still: no spinner turns, and no part of the view moves with time. */
  reducedMotion = false;
  /** The mode of the terminal, dark or light, as it said it last, which no file keeps. */
  terminalMode: ThemeMode | null = null;
  notice = "";
  private saved = "";
  constructor(readonly path = join(configDirectory(), "ui.json")) {
    try {
      const saved = JSON.parse(readFileSync(path, "utf8")) as {
        theme?: string;
        sidebar?: boolean;
        sidebarWidth?: number;
        sidebarStyle?: string;
        foldThreads?: boolean;
        foldRungs?: boolean;
        setupSeen?: boolean;
        view?: string;
        screenReader?: boolean;
        reducedMotion?: boolean;
      } | null;
      if (isThemeChoice(saved?.theme)) this.theme = saved.theme;
      if (typeof saved?.screenReader === "boolean") this.screenReader = saved.screenReader;
      if (typeof saved?.reducedMotion === "boolean") this.reducedMotion = saved.reducedMotion;
      if (typeof saved?.sidebar === "boolean") this.sidebar = saved.sidebar;
      // A file of an earlier version names the fold of what is over foldRungs.
      const fold = saved?.foldThreads ?? saved?.foldRungs;
      if (typeof fold === "boolean") this.foldThreads = fold;
      if (typeof saved?.setupSeen === "boolean") this.setupSeen = saved.setupSeen;
      if (isLifeView(saved?.view)) this.view = saved.view;
      if (typeof saved?.sidebarWidth === "number")
        this.sidebarWidth = Math.max(32, Math.min(72, saved.sidebarWidth));
      if (isRailStyle(saved?.sidebarStyle)) this.sidebarStyle = saved.sidebarStyle;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT")
        this.notice = `Could not read preferences at ${path}. Using the theme of the terminal for this run. The file is unchanged. Click to dismiss.`;
    }
    this.saved = this.serialize(this.theme);
  }
  /** Whether the view holds still, as reduced motion asks, and as the mode of a screen reader asks too. */
  get still(): boolean {
    return this.reducedMotion || this.screenReader;
  }
  private serialize(theme: ThemeChoice): string {
    return JSON.stringify({
      theme,
      sidebar: this.sidebar,
      sidebarWidth: this.sidebarWidth,
      sidebarStyle: this.sidebarStyle,
      foldThreads: this.foldThreads,
      setupSeen: this.setupSeen,
      view: this.view,
      screenReader: this.screenReader,
      reducedMotion: this.reducedMotion,
    });
  }
  save(theme: ThemeChoice = this.theme): void {
    const data = this.serialize(theme);
    if (data === this.saved) return;
    mkdirSync(dirname(this.path), { recursive: true });
    saveFile(this.path, data);
    this.theme = theme;
    this.saved = data;
    this.notice = "";
  }
}
