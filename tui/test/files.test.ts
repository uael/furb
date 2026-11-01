import { dlopen, FFIType, type Pointer, ptr } from "bun:ffi";
import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { projectFiles } from "../src/files.ts";

/** Drop from this process the privileges to back up and restore files, and give the function that sets them back.
 * An administrator of Windows that holds them enabled, as the user of a runner of CI does, reads a folder whatever
 * its access list denies. A child process inherits the privileges of this one as they stand when it starts. */
function withoutBackup(): () => void {
  const kernel = dlopen("kernel32.dll", {
    GetCurrentProcess: { args: [], returns: FFIType.ptr },
    CloseHandle: { args: [FFIType.ptr], returns: FFIType.i32 },
  });
  const advapi = dlopen("advapi32.dll", {
    OpenProcessToken: { args: [FFIType.ptr, FFIType.u32, FFIType.ptr], returns: FFIType.i32 },
    LookupPrivilegeValueW: { args: [FFIType.ptr, FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    AdjustTokenPrivileges: {
      args: [FFIType.ptr, FFIType.i32, FFIType.ptr, FFIType.u32, FFIType.ptr, FFIType.ptr],
      returns: FFIType.i32,
    },
  });
  const handle = new BigUint64Array(1);
  // 0x28 is the right to query the privileges of a token and to change them.
  if (!advapi.symbols.OpenProcessToken(kernel.symbols.GetCurrentProcess(), 0x28, ptr(handle)))
    throw new Error("The token of this process does not open.");
  const token = Number(handle[0]) as Pointer;
  const names = ["SeBackupPrivilege", "SeRestorePrivilege"];
  // A TOKEN_PRIVILEGES: the count, then for each privilege its LUID in two words and its state, where 0 is disabled.
  const dropped = new Uint32Array(1 + 3 * names.length);
  dropped[0] = names.length;
  names.forEach((name, index) => {
    if (
      !advapi.symbols.LookupPrivilegeValueW(
        null,
        ptr(Buffer.from(`${name}\0`, "utf16le")),
        ptr(dropped, 4 + 12 * index),
      )
    )
      throw new Error(`No privilege ${name}.`);
  });
  const before = new Uint32Array(dropped.length);
  const size = new Uint32Array(1);
  if (
    !advapi.symbols.AdjustTokenPrivileges(token, 0, ptr(dropped), before.byteLength, ptr(before), ptr(size))
  )
    throw new Error("The privileges of this process do not change.");
  return () => {
    const restored = advapi.symbols.AdjustTokenPrivileges(token, 0, ptr(before), 0, null, null);
    kernel.symbols.CloseHandle(token);
    if (!restored) throw new Error("The privileges of this process are not set back.");
  };
}

/** Take from every user the right to list a folder, and give the function that gives it back: by its mode on Unix,
 * and on Windows, where the mode holds no such right, by an entry that denies it in the access list of the folder. */
async function unreadable(folder: string): Promise<() => Promise<void>> {
  if (process.platform !== "win32") {
    await chmod(folder, 0o000);
    return () => chmod(folder, 0o700);
  }
  // S-1-1-0 is the group of every user, and RD the right to list a folder.
  const icacls = (...args: string[]) => {
    const done = Bun.spawnSync(["icacls", folder, ...args]);
    if (done.exitCode) throw new Error(`icacls failed: ${done.stdout.toString()}${done.stderr.toString()}`);
  };
  icacls("/deny", "*S-1-1-0:(RD)");
  const restore = withoutBackup();
  return async () => {
    restore();
    icacls("/remove:d", "*S-1-1-0");
  };
}

test("the project files leave out a folder that cannot be read, through rg and through the walk without rg", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-files-"));
  const project = join(directory, "project");
  let readable = async () => {};
  try {
    await mkdir(join(project, "src"), { recursive: true });
    await mkdir(join(project, "locked"));
    await writeFile(join(project, "src/a.py"), "a = 1\n");
    await writeFile(join(project, "README.md"), "# Read me\n");
    readable = await unreadable(join(project, "locked"));
    await expect(readdir(join(project, "locked"))).rejects.toThrow();
    expect(await projectFiles(project)).toEqual(["README.md", "src/a.py"]);
    // A process with no PATH finds no rg, so it walks the folders itself.
    const walk = Bun.spawn(
      [
        process.execPath,
        "-e",
        `import { projectFiles } from ${JSON.stringify(join(import.meta.dir, "../src/files.ts"))};
console.log(JSON.stringify(await projectFiles(${JSON.stringify(project)})));`,
      ],
      { env: { ...process.env, PATH: "" }, stdout: "pipe", stderr: "pipe" },
    );
    const [code, output, errors] = await Promise.all([
      walk.exited,
      new Response(walk.stdout).text(),
      new Response(walk.stderr).text(),
    ]);
    expect(errors).toBe("");
    expect(code).toBe(0);
    expect(JSON.parse(output)).toEqual(["README.md", "src/a.py"]);
  } finally {
    await readable();
    await rm(directory, { recursive: true, force: true });
  }
});
