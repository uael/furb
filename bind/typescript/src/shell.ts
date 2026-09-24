/** The POSIX shell that runs a command of the World: `/bin/sh`, and on Windows, which has none of its own, the `sh`
 * on PATH, such as the one of Git for Windows. */
export const shell = process.platform === "win32" ? "sh" : "/bin/sh";
