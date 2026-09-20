// A World of this machine for the binding tests, and the two helpers that drive a life on it.
//
// The tests drive one life of the real engine through the module the crate makes, so what they hold it to is the
// whole of the binding: a World of javascript behind the boundary, a Voice the host says into, a gate that reads
// the word of a rung, and every value that crosses between them.

import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
export const furb = require('../index.cjs')
const { Fact, Fault, Life, Shape, Tuple, Voice, line } = furb

/** The actors a life of the tests offers: the operator, and one model on one effort. */
export const ROSTER = new Tuple([
  new Tuple(['operator', new Tuple([]), 200000]),
  new Tuple(['m', new Tuple(['low']), 200000]),
])

/**
 * A World of this machine, for the tests: a directory, and a model that answers by a script.
 *
 * It is no part of the crate. What a life may touch is the host's to decide, so the crate says what a World is
 * and this is one, written small enough that a reader of the tests can hold the whole of it.
 */
export class Yard {
  constructor(at, voice, words = []) {
    this.at = at
    this.voice = voice
    this.words = [...words]
    this.read = []
    this.kept = []
    this.acts = new Map()
    this.asking = null
  }

  /** One fact, heard, and what the World says of it or the question it must ask first. */
  hears(fact) {
    if (fact.question && ['bash', 'wait', 'prompt'].includes(fact.kind)) this.acts.set(fact.about, fact)
    switch (fact.kind) {
      case 'stand':
        return new Fact('done', fact.about, '', [new Tuple([ROSTER, this.at, 'm/low'])])
      case 'clock':
      case 'chance':
        return new Fact('done', fact.about, '', [0.5])
      case 'read':
      case 'write':
        // Where a path resolves is the chain's to say, so it is asked before the disk is touched.
        this.asking = fact
        return { ask: 'cwd', on: fact.on }
      case 'start': {
        // A prompt of the operator is answered by whoever holds this World, so the yard answers it at once.
        const held = this.acts.get(fact.about)
        if (held && held.kind === 'prompt') this.voice.close(held.about, 'the operator says so')
        return null
      }
      case 'ask': {
        this.read.push(text(fact.words[2]))
        const word = this.words.length ? this.words.shift() : 'close(None)'
        this.voice.fact(new Fact('answer', fact.about, '', [new Tuple(['assistant', [word], null, []])]))
        return null
      }
      case 'keep':
        this.kept.push(line(fact.words[0]))
        return null
      default:
        return null
    }
  }

  /** The answer to the question the World last asked, and what it says now that it holds it. */
  answered(got) {
    const held = this.asking
    this.asking = null
    if (!held) return null
    // `resolve` is what `join` is in python and in rust: a chain that stands somewhere of its own replaces the
    // yard, and one that answers a relative path stands under it.
    const at = resolve(this.at, String(got ?? ''))
    if (held.kind === 'read') {
      const one = join(at, String(held.words[1]))
      if (!existsSync(one)) return new Fact('done', held.about, '', [new Fault('Refused', [`no file at ${one}`])])
      return new Fact('done', held.about, '', [new Shape('Text', { path: one, content: readFileSync(one, 'utf8') })])
    }
    const text = held.words[1]
    const one = join(at, String(text.fields.path))
    mkdirSync(dirname(one), { recursive: true })
    writeFileSync(one, text.fields.content)
    return new Fact('done', held.about, '', [new Shape('Text', { path: one, content: text.fields.content })])
  }
}

/** A gate of the tests, which refuses a word that holds BAD, as the harness of the suite does. */
export class Strict {
  /** What the gate finds against a word. Nothing at all means the word may run. */
  gate(word, ladder, shape) {
    return word.includes('BAD') ? ['BAD in rung'] : []
  }
}

/** Every word of a value, run together, which is how a test reads what a model was shown. */
export function text(got) {
  if (typeof got === 'string') return got
  if (Array.isArray(got)) return got.map(text).join(' ')
  if (got instanceof Tuple) return got.items.map(text).join(' ')
  if (got instanceof Shape) return Object.values(got.fields).map(text).join(' ')
  if (got instanceof Fault) return `${got.name} ${got.args.map(text).join(' ')}`
  if (got && typeof got === 'object') return Object.values(got).map(text).join(' ')
  return ''
}

/** The directory the chains of a life stand in. */
export function yard() {
  return mkdtempSync(join(tmpdir(), 'furb-js-'))
}

/** One life of the real engine, on a World of this machine and the words a model answers with. */
export function life(at, words = [], gate = null, record = null) {
  const voice = new Voice()
  const world = new Yard(at, voice, words)
  return [new Life(world, gate, record, voice), world]
}

/**
 * What an act came to, once the host has said everything it owes.
 *
 * A host says what it owes first, since a fact it is holding may be the very one that settles the act. Nothing
 * waits forever here: a life that never settles is a fault of the test and it says so.
 */
export function came(held, act, tries = 200) {
  for (let at = 0; at < tries; at++) {
    held.heard()
    const got = held.came(act)
    if (got !== null) return got
    held.waits(0.05)
  }
  throw new Error(`${act} never came to anything`)
}
