# Deployment inspection

`deployment inspect` looks up a deployed contract's instance entry and recovers
the executable WASM hash.

## How it works

1. The contract id (`C...` strkey) is decoded to its 32-byte hash.
2. A `LedgerKey::ContractData` key is built with `LedgerKeyContractInstance`.
3. `getLedgerEntries` is called through the configured transport.
4. The returned `LedgerEntryData` XDR is decoded; a WASM-backed instance yields
   `ContractExecutable::Wasm(hash)`, which is reported as hex.

Nothing about the RPC shape is invented; the key and entry are real Stellar XDR
types.

## Transports

The core does not hard-code a provider. Deployment uses the `Transport`
abstraction. This build ships a deterministic **fixture transport**; the live
HTTP transport is tracked as future work.

```bash
stellar-contract-observatory deployment inspect C... \
  --rpc-fixture rpc.json --network testnet
```

`rpc.json` maps method names to results:

```json
{
  "getLedgerEntries": {
    "entries": [
      { "key": "AAAA...", "xdr": "AAAA...", "lastModifiedLedgerSeq": 123 }
    ]
  }
}
```

An error response is expressed as `{ "__error": { "code": -32000, "message": "..." } }`.

## Deployed WASM retrieval

`deployment::deployed_wasm(client, wasm_hash_hex)` fetches the contract code
entry for a WASM hash and decodes the bytes. It builds a
`LedgerKey::ContractCode` key, calls `getLedgerEntries`, and decodes the
returned `LedgerEntryData`. It returns `None` when no code entry exists and
rejects hashes that are not 32 bytes or decode larger than the WASM size limit.

This enables interface comparison between a local artifact and the deployed
code (tracked as a follow-up).

## Reported fields

`contract_id`, `wasm_hash`, `last_modified_ledger_seq`,
`live_until_ledger_seq`, and `found`.

## Limitations

- Only WASM-backed instances expose a hash; Stellar-asset contracts report no
  hash.
- No caching is performed.
- A live provider means a network call; keep such use out of unit tests.
