// A host of typescript, written against the declarations alone, so that a fault in them is a fault of this file.
import { Ask, Fact, Fault, Gate, Life, Reply, Shape, Show, Tuple, Value, Voice, World, line } from '..'

class Yard implements World {
  private readonly asked: Fact[] = []

  constructor(private readonly at: string, private readonly voice: Voice) {}

  hears(fact: Fact): Reply {
    const kind: string = fact.kind
    const on: string | null = fact.on
    const words: Value[] = fact.words
    if (fact.question) this.asked.push(fact)
    if (kind === 'stand') {
      const roster = new Tuple([new Tuple(['operator', new Tuple([]), 200000])])
      return new Fact('done', fact.about, '', [new Tuple([roster, this.at, 'm/low'])])
    }
    if (kind === 'read') {
      const ask: Ask = { ask: 'cwd', on: on ?? '', words: [] }
      return ask
    }
    if (kind === 'keep') {
      const kept: string = line(words[0])
      this.voice.fact(new Fact('kept', fact.about, '', [kept]))
      return []
    }
    return null
  }

  answered(got: Value): Reply {
    if (got instanceof Shape) {
      const path: Value = got.fields.path
      return [new Fact('done', String(path), '')]
    }
    if (got instanceof Fault) return got.refused() ? null : []
    if (got instanceof Show) return null
    return null
  }
}

class Strict implements Gate {
  gate(word: string, ladder: string[], shape: string): string[] {
    return word.includes('BAD') ? [`BAD in ${shape} at ${ladder.length}`] : []
  }
}

export function drives(at: string): Value {
  const voice = new Voice()
  const life = new Life(new Yard(at, voice), new Strict(), null, voice)
  const root: string = life.root
  const world: World = life.world
  void world
  const act = String(life.word(`prompt(int, 'count', on=${JSON.stringify(root)})`))
  while (life.came(act) === null) {
    life.heard()
    if (!life.waits(0.05)) break
  }
  return life.came(act)
}
