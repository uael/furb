// The reader of the clipboard of each system, under its name, and gh, for the tests of an image of the clipboard and of
// a share. A reader prints the image in the file clipboard beside it: as base64 under the names of AppKit and
// PowerShell, which print it so, and as its bytes under the others. gh keeps what it uploads in the file uploaded.json
// in the folder above its own, and prints the address of a gist.
import { readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";

const here = dirname(process.execPath);
const name = basename(process.execPath).replace(/\.exe$/, "");
if (name === "gh") {
  const args = process.argv.slice(2);
  const [markdown = "", html = ""] = args.slice(-2);
  writeFileSync(
    join(here, "..", "uploaded.json"),
    JSON.stringify({ args, markdown: readFileSync(markdown, "utf8"), html: readFileSync(html, "utf8") }),
  );
  console.log("https://gist.github.com/furb/test-link");
} else {
  const data = readFileSync(join(here, "clipboard"), "utf8");
  process.stdout.write(/^(osascript|powershell)$/.test(name) ? data : Buffer.from(data, "base64"));
}
