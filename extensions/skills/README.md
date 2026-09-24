# Skills for furb

The skills extension finds the skills of a project and of its user, and reads one into a chain. A skill is a folder
that holds a `SKILL.md` file, as the skills of Claude are. It is the official example of an extension that is not a
builtin: it has a python part, a part for a World in TypeScript, and a part for the TUI, and furb loads it as it
loads any extension.

## Play it

Name it in `config.json` of the config directory of the user, or in `.furb/config.json` of a project:

```json
{ "extensions": { "skills": { "npm": "@furb/skills" } } }
```

A path and a git remote name it too:

```json
{ "extensions": { "skills": { "git": "https://github.com/uael/furb", "path": "extensions/skills" } } }
```

It requires the files extension, which is on unless a config turns it off.

## What a model reads

- `skills(on="")` gives the skills that the World finds for a chain, each a `Skill` with its `name`, its
  `description` and the `path` of its `SKILL.md`. It tells the chain one line for each skill that is new or changed
  since the question before it, and one line for each skill that is gone. It tells nothing when nothing changed.
- `skill(name, show=HEAD, on="")` reads the `SKILL.md` of a skill into the chain, as `read` reads a file, and gives
  its `Text`.

The life word of the extension, `skills()`, runs in every life on each chain without a source, so a chain tells its
skills before its first ask, and a later life tells what changed. `skills.pyi` is the contract of the python part,
and `test/` proves each of its sentences on both engines.

## Where the World finds skills

The part for a World answers the `skills` question with plain data. It reads three folders, in this order, and a
name that an earlier folder holds wins:

1. `.furb/skills` of the project.
2. `.claude/skills` of the project.
3. `skills` of the config directory of the user.

Each folder in them that holds a `SKILL.md` file is a skill. The frontmatter of the file, between two lines `---`,
gives its `name` and its `description`; a skill with no name in its frontmatter takes the name of its folder. The
World reads the disk at each question, so a new skill shows at the next one.

## Commands of the TUI

<!-- commands:start -->

| Command | Action |
| --- | --- |
| `/skills` | List the skills that this project and your config hold |
| `/skill <name>` | Read a skill into this chain |
| `/reload-skills` | Tell this chain the skills that are new, changed or gone |

<!-- commands:end -->
