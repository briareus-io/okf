# Author guidance that fits the operation

Use this example when configuring a bundle's operating guidance. Keep invariants in mandatory
rules, optional explanation in reference rules, and multi-step procedures in named workflows.
Existing rules without a `tier` remain mandatory.

```yaml
manifest_version: 1
guidance:
  instructions: Preserve authored evidence and unrelated changes.
  provenance:
    generator: example-publisher
  rules:
    - id: modeling-invariants
      operations: [add, edit, mv, rm, changeset plan, changeset apply]
      instructions: |
        Preserve ownership boundaries and distinguish current from proposed behavior.
        When changing type or relationships, inspect the relevant model before editing.
    - id: modeling-examples
      tier: reference
      description: Examples of ownership boundaries and model changes.
      operations: [add, edit, mv, rm, changeset plan, changeset apply]
      instructions_file: .okf-bundle/modeling-examples.md
workflows:
  remodel:
    description: Review and change the conceptual model.
    instructions_file: .okf-bundle/remodel.md
    resources: [.okf-bundle/model-review-checklist.md]
```

Create the referenced files before validating this example. `provenance` is optional publisher
metadata on global guidance, rules, or workflows; context returns it in `origin.provenance`.
Put generator identity there instead of prefixing every instruction body with boilerplate.
The CLI preserves authored instruction bytes, including HTML comments.

For a prose edit, request `bundle context --operation edit --stats --json`. The invariant is
mandatory and delivered in full; examples are a reference stub. Expand them with
`bundle context --rule modeling-examples --json` when useful. An expanded reference remains
inspected material. Selecting `--workflow remodel` loads that procedure but does not execute it.
Keep a useful description on each reference so the reader can decide whether to expand it.

Use `--guidance-budget 4096` to report whether delivered mandatory instructions exceed an
illustrative byte budget. This is advisory, never truncates instructions, and does not fail the
command. Measure a representative task mix before choosing a local budget. Byte counts are not
model-token estimates; omitted bodies are not read just to measure their size.

`--settings-detail compact` keeps the union of relevant setting groups and operational defaults,
with explicit omission reasons. It is independent of `--all`, which expands all guidance for
inspection and cannot issue an execution token. Unknown operation dependencies and selected
workflows conservatively retain all setting groups.
