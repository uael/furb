// A console of its own for one command on Windows, which sends Ctrl+Break to every process of a console. The command
// runs in this console with the pipes of this process, and Ctrl+Break goes to the console once the file at the first
// argument exists. This process exits with the exit code of the command.
import { dlopen, FFIType } from "bun:ffi";
import { existsSync } from "node:fs";

const { symbols } = dlopen("kernel32.dll", {
  FreeConsole: { args: [], returns: FFIType.i32 },
  AllocConsole: { args: [], returns: FFIType.i32 },
  GenerateConsoleCtrlEvent: { args: [FFIType.u32, FFIType.u32], returns: FFIType.i32 },
});
const CTRL_BREAK_EVENT = 1;
const [trigger = "", ...command] = process.argv.slice(2);
symbols.FreeConsole();
if (!symbols.AllocConsole()) throw new Error("This process could not make a console of its own.");
// This process hears the Ctrl+Break too, and lives on to give the exit code of the command.
process.on("SIGBREAK", () => {});
const child = Bun.spawn(command, { stdin: "inherit", stdout: "inherit", stderr: "inherit" });
let exited = false;
void child.exited.then(() => {
  exited = true;
});
while (!exited && !existsSync(trigger)) await Bun.sleep(20);
if (!exited && !symbols.GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, 0))
  throw new Error("Ctrl+Break was not sent.");
process.exit(await child.exited);
