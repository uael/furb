// The window and the host of a life: the socket that carries the parts of the host and the calls of the window, and
// what the views say in words. The window of the app opens the socket of the host that its core started; a browser
// opens the one that the address of the page names, as http://localhost:5174/?host=ws://...

import type { Answer, Said } from "./wire.ts";

/** Whether the UI runs in the window of the app, whose core started the host, or in a browser. */
export const tauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** A command of the core of the app, in its window; nothing in a browser. */
export async function core<T>(name: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!tauri) return undefined;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args);
}

/** The address of the socket of the host: the one that the core of the app gives, or the one that the page names. */
async function address(): Promise<string> {
  if (tauri) return (await core<string>("host")) ?? "";
  return new URLSearchParams(window.location.search).get("host") ?? "";
}

/** How many milliseconds the window waits before it opens the socket again, once it closed. */
const AGAIN = 1000;

let socket: WebSocket | undefined;
let calls = 0;
const waiting = new Map<number, { resolve(value: unknown): void; reject(error: Error): void }>();

/** The socket of the host, opened, and opened again each time that it closes until the returned stop. The window
 * hears each word of the host, and whether the socket is open. */
export function connect(hear: (said: Said) => void, open: (open: boolean) => void): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const start = async () => {
    const url = await address().catch(() => "");
    if (stopped) return;
    if (!url) {
      open(false);
      timer = setTimeout(() => void start(), AGAIN);
      return;
    }
    const one = new WebSocket(url);
    socket = one;
    one.onopen = () => open(true);
    one.onmessage = (event) => {
      const said = JSON.parse(String(event.data)) as Said | Answer;
      if (!("id" in said)) return hear(said);
      const call = waiting.get(said.id);
      waiting.delete(said.id);
      if (said.error !== undefined) call?.reject(new Error(said.error));
      else call?.resolve(said.value);
    };
    one.onclose = () => {
      if (socket === one) socket = undefined;
      for (const call of waiting.values()) call.reject(new Error("The host of the life is gone."));
      waiting.clear();
      open(false);
      if (!stopped) timer = setTimeout(() => void start(), AGAIN);
    };
  };
  void start();
  return () => {
    stopped = true;
    clearTimeout(timer);
    socket?.close();
  };
}

/** A call of the host, and what it comes to. */
export function call<T = unknown>(name: string, ...args: unknown[]): Promise<T> {
  const open = socket;
  if (!open || open.readyState !== WebSocket.OPEN)
    return Promise.reject(new Error("The host of the life is not connected."));
  const id = ++calls;
  return new Promise<T>((resolve, reject) => {
    waiting.set(id, { resolve: resolve as (value: unknown) => void, reject });
    open.send(JSON.stringify({ id, name, args }));
  });
}

/** The words a state reads as, for a tip. */
export type State = "asks" | "failed" | "paused" | "runs" | "waits" | "done" | "rest";
export const stateWords: Record<State, string> = {
  asks: "Waits for you",
  failed: "Failed",
  paused: "Paused",
  runs: "Runs",
  waits: "Waits on another program",
  done: "Done",
  rest: "At rest",
};

/** The model of an actor without its provider: claude-cli:sonnet/high reads sonnet, high. */
export function actorName(actor: string): string {
  const [model = actor, effort] = (actor.split(":").at(-1) ?? actor).split("/");
  return effort ? `${model}, ${effort}` : model;
}

/** Dollars as a person reads them. */
export function dollars(value: number): string {
  return `$${value.toFixed(2)}`;
}
