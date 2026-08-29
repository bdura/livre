---
title: Derive macros have unimplemented!() for unsupported data structures
severity: medium
---

The derive macros in `livre-derive/src/` contain `unimplemented!()` panics for certain data structure patterns that they don't yet support:

1. **`extract.rs`** (line ~57): `unimplemented!()` for `Data::Struct` with `Fields::Named` (structs with named fields)
2. **`from_raw_dict.rs`** (line ~51): `unimplemented!()` for `Data::Enum` and `Data::Union`
3. **`build_from_raw_dict.rs`** (line ~74): `unimplemented!()` for `Data::Enum` and `Data::Union`
4. **`utilities/attr.rs`** and **`utilities/extraction.rs`**: Additional `unimplemented!()` cases

**Location**: 
- `livre-derive/src/extract.rs`
- `livre-derive/src/from_raw_dict.rs`
- `livre-derive/src/build_from_raw_dict.rs`
- `livre-derive/src/utilities/attr.rs`
- `livre-derive/src/utilities/extraction.rs`

**Impact**: Users cannot derive `Extract`, `FromRawDict`, or `BuildFromRawDict` for:
- Structs with named fields (only tuple structs are supported)
- Enum types
- Union types

This limits the flexibility of the derive macros and forces users to write manual implementations for these common patterns.

**Reproduction**: Try to derive any of these macros on a struct with named fields or an enum.

**Suggested fix**: Implement support for named fields in structs, and for enum types. The enum case is particularly important as PDF dictionaries often use enum-like patterns.
