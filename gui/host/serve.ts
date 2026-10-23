// The socket of the host: one WebSocket on 127.0.0.1, which only a window that holds its token opens. Each window
// takes the parts of the host and sends its calls, and the host answers each call once, by its number.

import type { Answer } from "../src/wire.ts";
import type { Host } from "./host.ts";

/** A host served on a port of 127.0.0.1, any free one by default, and the address that a window opens. */
export function serve(host: Host, port = 0): { url: string; stop(): void } {
  const token = crypto.randomUUID();
  const server = Bun.serve<{ leave?: () => void }>({
    hostname: "127.0.0.1",
    port,
    fetch(request, server) {
      // A page of any origin can open a socket to 127.0.0.1, so the token alone lets a window in.
      if (new URL(request.url).searchParams.get("token") !== token)
        return new Response("This socket needs its token.", { status: 403 });
      if (server.upgrade(request, { data: {} })) return undefined;
      return new Response("The host speaks over a WebSocket.", { status: 426 });
    },
    websocket: {
      open(socket) {
        socket.data.leave = host.connect((said) => socket.send(JSON.stringify(said)));
      },
      async message(socket, message) {
        let call: { id: number; name: string; args?: unknown[] };
        try {
          call = JSON.parse(String(message));
        } catch {
          return;
        }
        let answer: Answer;
        try {
          answer = { id: call.id, value: (await host.call(call.name, call.args ?? [])) ?? null };
        } catch (error) {
          answer = { id: call.id, error: error instanceof Error ? error.message : String(error) };
        }
        socket.send(JSON.stringify(answer));
      },
      close(socket) {
        socket.data.leave?.();
      },
    },
  });
  return { url: `ws://127.0.0.1:${server.port}/?token=${token}`, stop: () => void server.stop(true) };
}
