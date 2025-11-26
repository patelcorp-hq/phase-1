# Phase 1: Connect to Data

Basic RPC connection and Raydium swap parser.

## What's Here

- RPC client for fetching Solana blocks and transactions
- Parser for Raydium V4 swaps (program ID: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`)
- Field extraction: signature, slot, timestamp, token addresses, amounts, signer
- ClickHouse insertion logic

## Status

- RPC connection established
- Transaction parsing working
- Successfully parsed and inserted 10 test swaps
- Data validated against Solscan
