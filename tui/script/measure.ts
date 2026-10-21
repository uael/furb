import { looming } from "../test/looming.ts";

// The cost of a frame and of a key on a life of a hundred chains, each of which asks the operator a question, so that
// a hundred messages flow on the map: bun tui/script/measure.ts.
await looming(
  async ({ app }) => {
    const median = (run: () => unknown, count = 20) => {
      const times: number[] = [];
      for (let n = 0; n < count; n++) {
        const start = performance.now();
        run();
        times.push(performance.now() - start);
      }
      return (times.sort((one, other) => one - other)[count >> 1] ?? 0).toFixed(1);
    };
    app.variant = "loom";
    console.log(`the loom: a frame takes ${median(() => app.render())} ms`);
    app.variant = "graph";
    app.graph.level = "threads";
    console.log(`the threads and the map: a frame takes ${median(() => app.render())} ms`);
    app.graph.level = "map";
    console.log(`the map: a frame takes ${median(() => app.render())} ms`);
    console.log(
      `the map: a key takes ${median(() => {
        void app.graph.move([0, 1]);
        app.render();
      })} ms`,
    );
    process.exit(0);
  },
  { seed: "hundred" },
);
