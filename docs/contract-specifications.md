# Contract specifications

`spec` decodes the `contractspecv0` custom section into a normalized,
parser-independent model.

## Model

```text
ContractSpec
  functions:   SpecFunction { name, doc, inputs: [SpecParam], outputs: [SpecType] }
  structs:     SpecStruct   { name, doc, fields: [SpecField] }
  unions:      SpecUnion    { name, doc, cases: [SpecUnionCase] }
  enums:       SpecEnum     { name, doc, cases: [SpecEnumCase{name,value}] }
  error_enums: SpecErrorEnum{ ... }
  events:      SpecEvent    { name, doc, prefix_topics, params, data_format }
```

Types (`SpecType`) cover the full `SCSpecTypeDef` surface: primitives
(`val`, `bool`, `void`, `error`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`,
`u256`, `i256`, `timepoint`, `duration`, `bytes`, `string`, `symbol`,
`address`, `muxed_address`), composites (`option`, `result`, `vec`, `map`,
`tuple`, `bytes[n]`), and `udt` references.

## Ordering semantics

- Functions, structs, unions, enums, error enums, and events are canonicalized
  **by name**; declaration order is not semantic.
- Function parameters, struct fields, union cases, and enum cases preserve
  **declared order** because it is semantic for XDR encoding and the call ABI.

## Validation

`spec` also runs validation and reports issues:

- duplicate function names,
- duplicate user-defined type names,
- references to unknown types,
- more than one return type on a function,
- duplicate event names (warning).

## Supported and unsupported

Supported: all `SCSpecEntry` variants in the Stellar XDR schema used by this
project (`FunctionV0`, `UdtStructV0`, `UdtUnionV0`, `UdtEnumV0`,
`UdtErrorEnumV0`, `EventV0`).

If a future XDR schema adds an entry kind, decoding that stream will fail with a
structured error rather than silently dropping data.
