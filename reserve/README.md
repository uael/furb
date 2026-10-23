# The name furb on crates.io

`furb/` is the crate that holds the name `furb` on crates.io until the crate of furb can go there, which is when
monty stands on crates.io with what furb needs. It is version 0.0.0 and has no code. The owner publishes it once, by
hand, from a checkout of main:

```sh
cargo publish --manifest-path reserve/furb/Cargo.toml --token <token of crates.io>
```

The first release of furb on crates.io, from `.github/workflows/publish.yml`, then comes after it as 0.2.0 or later.
