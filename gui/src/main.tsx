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
await Promise.all(faces.map((face) => document.fonts.load(face))).catch(() => undefined);

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
