import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";
import "./styles.css";

// The faces load before the first frame, so the code measures its lines once and right.
const faces = [
  '400 13px "Atkinson Hyperlegible Next"',
  '600 13px "Atkinson Hyperlegible Next"',
  '400 13px "Atkinson Hyperlegible Mono"',
];
// A load that stays pending would hold the first render for ever, and the page would never ask the core for its host,
// so the wait ends after a bound.
const FACES_WAIT = 3000;
await Promise.race([
  Promise.all(faces.map((face) => document.fonts.load(face))).catch(() => undefined),
  new Promise((done) => setTimeout(done, FACES_WAIT)),
]);

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
