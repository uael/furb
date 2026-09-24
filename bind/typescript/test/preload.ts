import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// No config and no cache of the developer reach a test: the config directory and the cache directory of the
// extensions are directories of this run, empty.
const home = mkdtempSync(join(tmpdir(), "furb-test-home-"));
process.env.FURB_CONFIG_DIR = join(home, "config");
process.env.FURB_CACHE_DIR = join(home, "cache");
