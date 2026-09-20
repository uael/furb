# furb for javascript

One life of the engine, in the sandbox, with a World you write in javascript.

[furb](https://github.com/uael/furb) runs the engine in monty and gives its surface to a host. This is that
surface for a host of javascript. The word of a model runs where the engine runs and reaches nothing of yours:
the one way out is the World you hand over, and every value that crosses is plain data.

```js
import { Fact, Life, Tuple, Voice } from '@uael/furb'

class Yard {
  hears(fact) {
    if (fact.kind === 'stand') {
      const roster = new Tuple([new Tuple(['operator', new Tuple([]), 200000])])
      return new Fact('done', fact.about, '', [new Tuple([roster, '/yard', 'm/low'])])
    }
    return null
  }
  answered(got) {
    return null
  }
}

const life = new Life(new Yard(), null, null, new Voice())
console.log(life.root) // chain://operator.1
console.log(life.word(`cwd(on=${JSON.stringify(life.root)})`)) // /yard
```

`index.d.ts` is the whole surface, and every name in it says what it is for.

## What a host writes

A World hears every fact and answers it: nothing at all, one fact, an array of facts to say in order, or an
`{ ask, on, words }`, which is a question of the engine that comes back through `answered`. What the World starts
and does not finish there, it says later into the `Voice` it holds, and the life hears all of it, in the order it
was said, at every `Life.heard`.

A gate is what reads the word of a rung before it runs. It is yours to write too, and a host that reads no python
leaves it out: the word then runs, raises where it stands, and the engine asks the model again.

A World answers where it is asked, so `hears` and `answered` give a value and not a promise. This costs nothing:
the work that waits, which is a model, a command or a person, is what the Voice is for, and a World says that
work into the Voice whenever it finishes.

## Build

```sh
uv run python script/bound.py
```

The module is one library for one platform, and node reads it by the name `furb.node`, so the rig builds it and
puts it under `target/node` by that name. It then runs the tests of both bindings.
