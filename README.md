# supply_chain

`supply_chain` is a test helper for snapshotting the output of
[`cargo-supply-chain`][cargo-supply-chain]. It makes changes to the publishers in a Rust project's
dependency graph visible during testing and code review.

## Installation

Install `cargo-supply-chain`:

```console
cargo install cargo-supply-chain
```

Add `supply_chain` as a development dependency:

```console
cargo add --dev supply_chain
```

## Usage

Add a test that specifies where the snapshot should be stored:

```rust
#[test]
fn supply_chain() {
    supply_chain::check("tests/supply_chain.json");
}
```

Create or update the snapshot by running the test with `BLESS` set:

```console
BLESS=1 cargo test supply_chain
```

Commit the generated snapshot. Subsequent test runs compare the current report with the committed
snapshot:

```console
cargo test supply_chain
```

If the report changes, inspect the diff. If the change is expected, rerun the test with `BLESS=1`
and commit the updated snapshot.

## Behavior

For each check, `supply_chain`:

1. On the first check in each process, runs `cargo supply-chain update --cache-max-age=0s`.
2. Runs `cargo supply-chain json --no-dev`.
3. Removes all `avatar` fields from the JSON report and pretty-prints it.
4. Compares the report with the stored snapshot, or updates the snapshot when `BLESS` is set to a
   value other than `0`.

The exit status of the update command is ignored, but the report command must succeed.

This crate provides a reviewable signal when a dependency's supply-chain metadata changes. It does
not determine whether a dependency or publisher is trustworthy.

## Known issues

Because Cargo can include weakly referenced optional dependencies in the resolved dependency graph
([Cargo issue #10801][cargo-10801]), the report can include dependencies that are not enabled. The
[`cargo_10801` fixture][cargo-10801-fixture] demonstrates this behavior: its `serialization`
feature enables `serde` and weakly requests the `time?/serde-well-known` feature without enabling
the optional `time` dependency. Although `time` is never built, the fixture's `supply_chain.json`
includes `time` and `time`'s own dependencies `itoa`, `libc`, and `num_threads`.

The fixture's `chrono` dependency is the control: it is likewise optional and not enabled, but no
feature mentions it, and it does not appear in the report.

## License

Licensed under either of the following, at your option:

- Apache License, Version 2.0
- MIT License

[cargo-10801-fixture]: fixtures/cargo_10801
[cargo-10801]: https://github.com/rust-lang/cargo/issues/10801
[cargo-supply-chain]: https://github.com/rust-secure-code/cargo-supply-chain
