# `jcal` an experimental `cal`

As of now, this is a work in progress. This is a `cal` replacement with the
initial goal of supporting the Jalali calendar (using
[`jelal`](https://crates.io/crates/jelal)).

There are differences between this and cal:
- Immature and hastily written (work in progress, ugly code warning)
- Supports Jalali
- Supports any day as the start of the week
- No reform is supported yet
- Week counting system (the first week for now is always the first week with all
  its 7 days in the year)
- No localization support for now
- Minor formatting and highlighting rules

There is also another long abandoned project `jcal` (C based) and has no active
forks. These projects are not related in any ways but this can be an improved
replacement. Moreover, all contributions are welcome.

# License

As defined in `Cargo.toml`.
