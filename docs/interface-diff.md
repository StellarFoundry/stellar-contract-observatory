# Interface diff

`diff` enumerates the differences between two canonical interfaces and assigns
each change a default severity. It never claims compatibility; that is
`compat`'s job.

## Change kinds

Functions: `function_added`, `function_removed`, `function_changed`.
Types: `type_added`, `type_removed`, `type_changed`.
Events: `event_added`, `event_removed`, `event_changed`.

## Default severity rules

| Change | Severity |
| ------ | -------- |
| Function added | non-breaking |
| Function removed | breaking |
| Function signature changed | breaking |
| Function documentation only | informational |
| Type added | non-breaking |
| Type removed | breaking |
| Type definition changed | breaking |
| Type documentation only | informational |
| Event added | non-breaking |
| Event removed | breaking |
| Event definition changed | breaking |

## Determinism

Changes are sorted by kind rank, then path, then detail. Running `diff` twice on
the same inputs yields byte-identical output.

## Output

Human mode prints one line per change with a marker (`!` breaking, `+`
non-breaking, `~` informational). JSON mode emits the full `InterfaceDiff`,
including `old`/`new` renderings for signature and type changes.

## What it does not do

- It does not decide compatibility. Use `compat`.
- It does not detect renames reliably; a rename appears as a removal plus an
  addition. Rename detection is future work.
