# TC spell-child edge artifact v1

Producer: `scripts/extract_tc_spell_children.py`. Pinned source: TrinityCore `a352b1fa`, with each class-script SHA256 validated against the supplied manifest. Root→script bindings come from the supplied census index, not guessed class names. Output refuses an existing path and is deterministic for identical inputs.

```text
python3 scripts/extract_tc_spell_children.py \
  --source-dir <pinned-class-script-directory> \
  --index <root-to-script-index.json> \
  --manifest <class-script-sha256-manifest.json> \
  --output <new-child-edges.json>
```

## JSON contract

- `schema_version`: integer1; `source_revision`: pinned revision.
- `source_files`: objects with `file` basename, `revision`, `sha256`.
- `index_sha256`, `manifest_sha256`: hashes of the exact binding/index inputs.
- `edges`: objects with `parent_spell_id`, positive `child_spell_id`, `file`, `script_class`, one-based `cast_line`, original spell `expression`, `argument_count`, `kind="tc_constant_cast"`.
- `unresolved_calls`: same call identity, without a guessed child; `reason` is `dynamic_spell_expression`, `unsupported_cast_overload` or `nonpositive_child_id`.
- `coverage`: counts of script bindings, source files, constant edges and unresolved calls.

Multiple script bindings or multiple calls may yield the same parent/child pair. Consumers can deduplicate graph edges while preserving all cited call sites. The extractor does not flatten transitive closure or deduce runtime conditions.

## Exact extraction coverage

|Construct|Handling|Proof|
|---|---|---|
|Ordinary target-first CastSpell/CastCustomSpell|Spell expression is argument2|Native enum fixture and literal5176 fixture|
|Three literal coordinates before the spell|Spell expression is argument4|Literal-coordinate133 fixture|
|Explicit enum integer and integer const/constexpr declarations|Resolve same-file constant IDs; conflicting definitions fail|Native Shield Charge385952→385953 fixture; native385953 effect96 then triggers385954 outside this TC-only artifact|
|Literal positive decimal/hex child IDs|Emit direct cited edge|Literal fixture|
|Dynamic TriggerSpell/function/variable expressions|Record unresolved; no invented child|Dynamic TriggerSpell fixture|
|Comments and ordinary quoted literals|Blank while preserving offsets/line numbers; not calls|Comment/string counterexamples in the fixture|
|Arithmetic enum expressions, implicit enum increments, runtime ternaries, general C++ overload/template resolution|Not evaluated; affected expressions remain unresolved|No completeness claim|
|Missing/multiple script definitions, wrong revision/hash, malformed delimiters|Fail explicitly; no output artifact|Hash-mutation fixture; other input errors are not silently dropped|

This is a bounded clean-room source-data extractor, not a C++ interpreter. Raw strings/preprocessor condition evaluation are not certified. The output describes possible constant child calls, not proof those branches execute, native dispatch exists, targets are legal or damage is effective. A missing edge is not proof a spell has no children. Consumers must retain unresolved calls and source limits when classifying native behavior.

## Shared artifact

Canonical generated artifact: sibling `game-engine/data/diagnostics/tc-script-child-edges-a352b1fa.json`. Runtime proof and exact command live under `game-engine/data/diagnostics/dps-parity-2026-10-21/`. Do not modify its pinned C++ sources or existing census index to make extraction pass.
