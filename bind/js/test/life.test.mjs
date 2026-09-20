// One life of the real engine, driven from javascript through the module the crate makes.
//
// Every test here holds the whole boundary at once: a World of javascript hears the facts, a Voice says into the
// life what the host finished later, and what crosses between them is the plain form and nothing of the engine.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { test } from 'node:test'

import { Strict, Yard, came, furb, life, yard } from './yard.mjs'

const { Fact, Fault, Life, Shape, Tuple, Voice } = furb

test('a life opens on its root and answers what the root stands on', () => {
  const at = yard()
  const [held, world] = life(at)
  assert.equal(held.root, 'chain://operator.1')
  assert.equal(held.word(`cwd(on=${JSON.stringify(held.root)})`), at)
  assert.equal(held.word(`len(turns(on=${JSON.stringify(held.root)}))`), 1)
  assert.equal(held.world, world)
})

test('a World serves a read and a write', () => {
  const at = yard()
  const [held] = life(at)
  const root = JSON.stringify(held.root)

  const wrote = held.word(`write(Text('a.txt', 'one\\ntwo\\n'), on=${root})`)
  assert.equal(wrote.fields.content, 'one\ntwo\n')
  assert.equal(readFileSync(join(at, 'a.txt'), 'utf8'), 'one\ntwo\n')

  const got = held.word(`read('a.txt', on=${root})`)
  assert.equal(got.fields.path, join(at, 'a.txt'))
  assert.deepEqual(got.fields.content.split('\n').slice(0, 2), ['one', 'two'])
})

test('a read the World refuses raises in the caller', () => {
  const [held] = life(yard())
  assert.throws(
    () => held.word(`read('none.txt', on=${JSON.stringify(held.root)})`),
    (one) => one.message.includes('no file at'),
  )
})

test('a model answers a prompt and the Kernel runs the word it wrote', () => {
  const [held, world] = life(yard(), ["close(len(read('a.txt').lines))"])
  const root = JSON.stringify(held.root)
  held.word(`write(Text('a.txt', 'one\\ntwo\\nthree\\n'), on=${root})`)
  const act = held.word(`prompt(int, 'count the lines', on=${root})`)
  assert.equal(came(held, act), 3)
  assert.ok(world.read[0].includes('count the lines'))
})

test('the gate refuses a word and the engine asks the model again', () => {
  const [held, world] = life(yard(), ['close(BAD)', 'close(7)'], new Strict())
  const act = held.word(`prompt(int, 'count', on=${JSON.stringify(held.root)})`)
  assert.equal(came(held, act), 7)
  assert.ok(world.read[1].includes('BAD in rung'))
})

test('the operator answers a prompt by closing it', () => {
  const [held] = life(yard())
  const act = held.word(`prompt(str, 'say a word', to='operator', on=${JSON.stringify(held.root)})`)
  assert.equal(came(held, act), 'the operator says so')
})

test('what the World keeps opens a second life', () => {
  const at = yard()
  const [held, world] = life(at, ['close(1)'])
  const act = held.word(`prompt(int, 'count', on=${JSON.stringify(held.root)})`)
  assert.equal(came(held, act), 1)

  const [again] = life(at, [], null, world.kept.join('\n'))
  assert.equal(again.root, held.root)
})

test('a Voice is heard by one life', () => {
  const at = yard()
  const voice = new Voice()
  new Life(new Yard(at, voice), null, null, voice)
  assert.throws(
    () => new Life(new Yard(at, voice), null, null, voice),
    (one) => one.message.includes('a Voice is heard by one life'),
  )
})

test('a fault of the World stands', () => {
  const at = yard()
  const voice = new Voice()
  class Broken extends Yard {
    hears(fact) {
      if (fact.kind === 'clock') throw new Error('the clock of this World is broken')
      return super.hears(fact)
    }
  }
  const held = new Life(new Broken(at, voice), null, null, voice)
  assert.throws(
    () => held.word(`clock(on=${JSON.stringify(held.root)})`),
    (one) => one.message.includes('the clock of this World is broken'),
  )
})

test('a value a life holds none of is refused', () => {
  const at = yard()
  const voice = new Voice()
  class Odd extends Yard {
    hears(fact) {
      if (fact.kind === 'clock') return new Fact('done', fact.about, '', [() => 1])
      return super.hears(fact)
    }
  }
  const held = new Life(new Odd(at, voice), null, null, voice)
  assert.throws(
    () => held.word(`clock(on=${JSON.stringify(held.root)})`),
    (one) => one.message.includes('a life holds no Function'),
  )
})

test('every plain value crosses both ways', () => {
  const [held] = life(yard())
  assert.equal(held.word('None'), null)
  assert.equal(held.word('True'), true)
  assert.equal(held.word('3'), 3)
  assert.equal(held.word('1.5'), 1.5)
  assert.equal(held.word("'one'"), 'one')
  assert.deepEqual(held.word("[1, 'two']"), [1, 'two'])

  const pair = held.word("(1, 'two')")
  assert.ok(pair instanceof Tuple)
  assert.deepEqual(pair.items, [1, 'two'])

  assert.deepEqual(held.word("{'a': 1}"), { a: 1 })

  const text = held.word("Text('a.txt', 'one')")
  assert.ok(text instanceof Shape)
  assert.equal(text.name, 'Text')
  assert.deepEqual(text.fields, { path: 'a.txt', content: 'one', before: null })

  const fault = held.word("Refused('no')")
  assert.ok(fault instanceof Fault)
  assert.equal(fault.name, 'Refused')
  assert.deepEqual(fault.args, ['no'])
  assert.equal(fault.refused(), true)
})
