# The names furb and furb-cli on crates.io

`furb/` is the crate that holds the name `furb` on crates.io until the crate of furb can go there, which is when
monty stands on crates.io with what furb needs. It is version 0.0.0 and has no code. The owner published it once, by
hand, from a checkout of main:

```sh
cargo publish --manifest-path reserve/furb/Cargo.toml --token <token of crates.io>
```

furb 0.0.0 stands on crates.io, and an empty furb-cli 0.0.0 holds the name of the command line too. On crates.io,
each crate names `.github/workflows/publish.yml` as a trusted publisher, so the first release of furb on crates.io
comes after them with no token.
