import { dlopen, FFIType, type Pointer, ptr } from "bun:ffi";
import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { FilePicker } from "furb";
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

test("the project files leave out a folder that cannot be read, .git and what .gitignore hides, and list dot files", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-files-"));
  const project = join(directory, "project");
  let readable = async () => {};
  try {
    await mkdir(join(project, "src"), { recursive: true });
    await mkdir(join(project, "locked"));
    await mkdir(join(project, ".git"));
    await mkdir(join(project, "target"));
    await writeFile(join(project, "src/a.py"), "a = 1\n");
    await writeFile(join(project, "README.md"), "# Read me\n");
    await writeFile(join(project, ".env"), "A=1\n");
    await writeFile(join(project, ".gitignore"), "target/\n");
    await writeFile(join(project, ".git/HEAD"), "ref: refs/heads/main\n");
    await writeFile(join(project, "target/built.rs"), "");
    readable = await unreadable(join(project, "locked"));
    await expect(readdir(join(project, "locked"))).rejects.toThrow();
    expect(await projectFiles(project)).toEqual([".env", ".gitignore", "README.md", "src/a.py"]);
  } finally {
    await readable();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a picker in a tree of 50000 files takes each key at once, and its rows come as its walk and its match end", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-picker-"));
  try {
    for (let folder = 0; folder < 500; folder++) {
      await mkdir(join(directory, `folder${folder}`));
      await Promise.all(
        Array.from({ length: 100 }, (_, file) =>
          writeFile(join(directory, `folder${folder}`, `file${file}.ts`), ""),
        ),
      );
    }
    // The keys come as fast as a test types them, from the open of the picker, while its walk runs.
    const picker = new FilePicker(directory);
    const waited: number[] = [];
    let typed = "";
    let found = picker.rows(50);
    for (const key of "folder42/file7") {
      typed += key;
      const began = performance.now();
      found = picker.find(typed, 50);
      waited.push(performance.now() - began);
    }
    // A walk of the tree takes about 300 ms and a match of it about 20 ms, so a key that waited for one goes past this.
    expect(Math.max(...waited)).toBeLessThan(250);
    for (const until = Date.now() + 20_000; found.running && Date.now() < until; await Bun.sleep(10))
      found = picker.rows(50);
    expect(found.paths[0]).toBe("folder42/file7.ts");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}, 60_000);
