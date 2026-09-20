// The module a host requires, which is the library cargo made.
//
// A published package holds the library for one platform beside this file, and a checkout holds what cargo built
// under `target`. Both are looked for, in that order, so a host reads the same module either way.

const { existsSync } = require('node:fs')
const { join } = require('node:path')

const HERE = __dirname
const ROOT = join(HERE, '..', '..')
/** Where the library is, or nothing at all.
 *
 * Node reads a native module by its name, and cargo names a library after the platform it built for, so a
 * checkout holds the built one under `target/node` by the name node reads it as. `script/bound.py` puts it there.
 */
function found() {
  for (const at of [HERE, join(ROOT, 'target', 'node')]) {
    const one = join(at, 'furb.node')
    if (existsSync(one)) return one
  }
  return null
}

const at = found()
if (at === null) {
  throw new Error(`furb for javascript is not built: \`uv run python script/bound.py\` builds it, and nothing of it is under ${ROOT}`)
}

module.exports = require(at)
