TODO

## Git hooks

[hk](https://hk.jdx.dev) formats the staged snapshot of Rust files on commit and
restores unstaged edits afterward. It is a separate CLI, not a crate dependency.
Install it once, then register the hook once per clone:

```sh
brew install hk
hk install
```

