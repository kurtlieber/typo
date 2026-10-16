# mitex Typst specs

`specs/` is copied unchanged from [mitex](https://github.com/mitex-rs/mitex)
at tag `0.2.7` (`packages/mitex/specs/`), under the Apache License 2.0 in
`LICENSE`.

The mitex crate converts LaTeX into Typst that calls the functions these files
define (`mitex-scope`); typo evaluates that Typst with typst to draw math. The
crate's own command table is generated from the same files when it builds,
which is why the build needs the `typst` command, and why these files must
stay at the same tag as the `mitex` dependency in `Cargo.toml`.
