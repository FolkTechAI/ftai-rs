# Test Fixtures

These `.ftai` files and the Python reference parser are vendored from
[FolkTechAI/ftai-spec](https://github.com/FolkTechAI/ftai-spec) under
the same Apache-2.0 license. Updates to those upstream files should
be re-vendored periodically.

## Files

- `sample_valid.ftai` — canonical sample from `parsers/python/sample_valid.ftai`
- `pass_*.ftai` — passing test vectors from `tests/vectors/pass/`
- `dummy_test.ftai` — passing CI smoke test fixture
- `parseftai_linter.py` — Python reference linter (used by `parity_python.rs`)
- `parseftai_json_adapter.py` — small wrapper that emits an FTAI document as
  JSON for byte-comparison with `ftai::parse(_)` output.

The parity test in `tests/parity_python.rs` is `#[ignore]` by default
because it requires `python3` in `PATH`. Run it with:

```
cargo test --test parity_python -- --ignored
```
