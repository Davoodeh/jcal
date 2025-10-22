# `jdate` an experimental `date`

As of now, this is a work in progress. This is a `date` replacement with the
initial goal of supporting the Jalali calendar (using
[`jelal`](https://crates.io/crates/jelal)).

There are differences between this and cal:
- Supports Jalali
- No resolution check or adjustment support
- No localization support for now
- No setting operations defined
- Parsing of the date may slightly vary
- Immature (work in progress)

There is also another long abandoned project `jcal` (C based) which provides a
`jdate` binary but has no active forks. These projects are not related in any
ways but this can be an improved replacement. Moreover, all contributions are
welcome.

# License

As defined in `Cargo.toml`.
