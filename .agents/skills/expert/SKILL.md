---
name: expert
description: The skills that each task of furb runs, and the rules of the owner that win over them. Run it with skill("ponytail") before you design or write, and with skill("ponytail-review") before you review a design note or a diff.
---

# Work as an expert

> Je veux uniquement du code comme si c'était un expert qui l'avait écrit, comme si c'était un expert qui l'avait
> review.
>
> EN: I want only code as if an expert had written it, as if an expert had reviewed it.

Each quote is from the owner: his words, or his instructions to agents. The English under a French quote is a
translation.

## The skills of each task

- To write: `ponytail` and `codebase-design`.
- To review a design note or a diff: `ponytail-review` and `code-review`.
- For architecture and convergence: `improve-codebase-architecture` and the rules of Ousterhout,
  `a-philosophy-of-software-design`.
- For a refactor: the rules of Fowler, `refactoring`.
- For the front end: the commands `critique`, `distill`, `audit` and `polish` of `impeccable`, with `opentui` and
  `frontend-design`.
- To write a test: `tdd`, below the suite of `AGENTS.md` and the skills `testing` and `test-audit`.

The rules of the owner below win over each of these skills, and `AGENTS.md` wins where it says more. A review reads a
design note as the diff that it will make, and gives one report in the form of `ponytail-review`: the axes and the
smells of `code-review` give findings in it as each other check does.

## 1. Read the mechanism of furb first

> And if we go that direction, it has to be an extension thing, not a new mechanism.

> Read the whole relevant artifact before deriving its design.

This is step 2 of the ladder of `ponytail` in furb. Before you write code, find the mechanism of furb that does the
nearest thing: the engine `src/furb/engine.py` and its contract `src/furb/engine.pyi`, the Kernel
`src/furb/kernel.py`, the World in `src/world/`, an extension or a skill. Read it, and name it in your design note or
your pull request. When no mechanism fits, say why.

## 2. No shortcut, no deferral

> No hacks or workaround, this is the promise

> On repousse pas, on défère pas quelque chose parce que c'est un peu plus compliqué. Non, on va vers la bonne
> solution.
>
> EN: We do not push back, we do not defer something because it is a bit more complicated. No, we go to the right
> solution.

> A loud unfinished path is acceptable only for a real, reported blocker. Its error states what is missing and why.

So the rule `shortcut:` of `ponytail` does not apply. Make the right solution now, also when it is hard, and write no
`shortcut:` comment. In a review or an audit, a `shortcut:` or `ponytail:` comment is a finding of **Must fix**, not a
decision, and the ledger of `ponytail-debt` stays empty. A finding on a change is fixed in that change, whatever its
grade: "Fix in next pass" and "Fix if time permits" do not hold. The refactor that a change needs is part of that
change, also in the loop of `tdd`.

## 3. Converge

> Ce qui fait que du coup les kits et les skills, ça devient la même extension et on merge les deux et on évite
> d'avoir deux trucs séparés
>
> EN: So kits and skills become the same extension: we merge the two and avoid having two separate things

> Repair, move, or delete the existing implementation. Do not create a parallel implementation to avoid fixing it.

Do one thing in one way. A change that adds a second way to do a thing deletes the first way in the same change, or
the review refuses it. Before a change deletes a public capability, as a name of the contract or of a door, get the
consent of the owner:

> Obtain explicit consent for unapproved public capability removals, new cross-cutting costs, and additional spend.

## 4. The size of a diff

> Une code base saine, c'est une code base où les éléments ils s'imbriquent ensemble, où tout se compose de manière
> cohérente et consistante, et où du coup il n'y a pas besoin de beaucoup de code pour exprimer quelque chose
> d'extrêmement puissant.
>
> EN: A healthy code base is one where the parts fit together, where everything composes coherently and consistently,
> so that it needs little code to express something extremely powerful.

> Same `ear` as above, if this is common pattern, please extract into a reusable function

> Meet size constraints through representation changes, without deleting capabilities, obscuring names, or hiding
> concepts through inlining.

A review compares the size of a diff with the size of its idea. A diff that is larger than its idea is a finding of
**Must fix**, and its case is the smaller diff that does the same thing. A copy of code that furb holds already is a
finding of **Must fix**: the fix uses that code and names its path. A smell of `code-review` with a concrete case is a
finding, not a judgement call.

## 5. The texts of others

`ponytail` and its five skills come from [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail);
`codebase-design`, `improve-codebase-architecture`, `code-review` and `tdd` from
[mattpocock/skills](https://github.com/mattpocock/skills); `a-philosophy-of-software-design` and `refactoring` from
[ciembor/agent-rules-books](https://github.com/ciembor/agent-rules-books); and `impeccable` from
[pbakaus/impeccable](https://github.com/pbakaus/impeccable). Each folder holds the texts that its author published at
the tag that its `metadata` names, as `gh skill install` puts them, and the license of its source. It holds only the
files that its task reads. Do not edit these texts: to take a later tag, install them again with `--pin` and keep the
same files. furb holds the texts alone, so these parts of them do not apply:

- Their scripts, plugins, hooks, commands and configs: run none. `impeccable` takes the path "Launcher unavailable"
  of its Setup, and a step that runs `scripts/impeccable` does not run, which the report says. The level of
  `ponytail` is full.
- The skills and the files that furb does not hold, as `grilling`, `domain-modeling`, `setup-matt-pocock-skills`,
  `HTML-REPORT.md` and the other commands of `impeccable`.
- A `GLOSSARY.md`, an ADR or an HTML report: write none. The design note and the pull request hold what they would
  hold.
- A question to the user: decide from the brief and the rulings of the owner, and name each decision in the pull
  request.

  > Ask only when an unresolved product decision or missing permission prevents a correct action.

## What stays

> A simplification request preserves capabilities and changes representation.

These rules cut nothing from the list "Never cut" of `ponytail`: validation at trust boundaries, the handling of
errors that prevents data loss, security and accessibility stay.
