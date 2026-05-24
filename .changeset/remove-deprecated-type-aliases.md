---
"emdash": minor
---

Removes deprecated and unused type aliases from the public API.

- Removes `ImageValue` (use `MediaValue` — identical type, migration is a find-replace).
- Removes `LoaderCollectionFilter` (was exported but never referenced anywhere).
- Deduplicates `ColumnType`: the definition is now canonical in `schema/types.ts`; `fields/types.ts` re-exports it from there instead of redefining it. No consumer impact.
