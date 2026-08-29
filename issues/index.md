# Issues

Potential issues detected while working on the project.

## Index

- [unimplemented-stream-filters](unimplemented-stream-filters.md) — **medium.** Multiple stream filters are parsed but return errors, preventing parsing of PDFs with non-FlateDecode compression.
- [derive-macro-unimplemented-cases](derive-macro-unimplemented-cases.md) — **medium.** Derive macros panic with unimplemented!() for named struct fields and enum types.
- [font-parsing-missing](font-parsing-missing.md) — **medium.** Font parsing not yet implemented (v0.6.0 roadmap item), preventing accurate character positioning.
- [text-object-indirection-fixme](text-object-indirection-fixme.md) — **low.** FIXME: Unnecessary indirection in TextObject text_buffer field.
- [map-value-dispatch-todo](map-value-dispatch-todo.md) — **low.** TODO: Replace catch-all with per-type dispatch in Map value parsing to reject invalid input.
- [delimiter-counter-todo](delimiter-counter-todo.md) — **low.** TODO: Evaluate need for u16 counter in delimiter parsing, u8 may suffice.
- [examples-use-panic](examples-use-panic.md) — **low.** Examples use panic!() instead of proper error handling, not demonstrating best practices.
