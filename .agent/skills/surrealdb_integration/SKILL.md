---
name: SurrealDB Integration
description: Guide and best practices for using SurrealDB 3.0 with rs-core/gen-model-fork
---

# SurrealDB 3.0 Integration Skills for rs-core/gen-model-fork

This skill provides a reference for using SurrealDB (v3.0+) within the current Rust project ecosystem (`rs-core`, `gen-model-fork`). It synthesizes existing documentation and codebase patterns.

## 1. Core Architecture & Setup

The project uses a custom fork of `surrealdb` (likely v3.0 compatible via `happydpc/surrealdb`).

-   **Global Client**: Accessed via `aios_core::SUL_DB`.
-   **Trait**: `aios_core::SurrealQueryExt` extends the client with helper methods (`query_take`, `query_response`).
-   **Data Types**:
    -   **`RefnoEnum`**: Primary key type, automatically converts to/from SurrealDB Record IDs (e.g., `pe:⟨123_456⟩`).
    -   **`SurrealValue`**: Derivable trait for structs mapping to DB results. **Do not use `serde_json::Value`**.

## 2. Basic Query Syntax (Surql 3.0)

### 2.1 SELECT & filtering
```sql
-- Select specific fields
SELECT refno, noun FROM pe WHERE deleted = false;

-- Select raw value (array of IDs)
SELECT VALUE id FROM pe WHERE noun IN ['BOX', 'CYLI'];

-- Select from specific records
SELECT * FROM [pe:⟨123_456⟩, pe:⟨789_000⟩];
```

### 2.2 Record IDs (Critical)
The project uses specific ID formats. Always use underscore `_` for complex refnos, not slash `/`.

-   **Format**: `table:⟨id_content⟩` (e.g., `pe:⟨12345_67890⟩`)
-   **Rust Helper**: `refno.to_pe_key()` generates the formatted ID string.
-   **Range Query**: `table:[start]..[end]` is preferred over `WHERE record::id() > ...`.

### 2.3 Graph Traversal (`->` / `<-`)
SurrealDB's graph navigation is heavily used.

```sql
-- Outgoing: "Has relation to..."
SELECT * FROM pe:⟨123⟩->inst_relate;

-- Incoming: "Is referenced by..."
SELECT VALUE in FROM pe:⟨123⟩<-pe_owner; -- Get children (entities that own this as parent)
```

## 3. Advanced Features (SurrealDB 3.0)

### 3.1 Recursive Path Queries (`@.{range}.field`)
This is the modern way to query deep hierarchies (descendants).

**Syntax**: `@.{RANGE + OPTIONS}.FIELD`

-   **Range**:
    -   `..` (Infinite)
    -   `3` (Exactly 3 levels)
    -   `1..5` (Levels 1 to 5)
-   **Options**:
    -   `collect` (Gather results into a flat array)
    -   `inclusive` (Include the starting node)

**Example**: Get all descendants recursively:
```sql
SELECT VALUE array::flatten(@.{..+collect}.children) FROM ONLY $root;
```

### 3.2 Custom Database Functions (`fn::`)
The project defines server-side functions to optimize performance (avoiding network roundtrips).

| Function | Purpose |
| :--- | :--- |
| `fn::collect_descendant_ids_by_types($root, $types, $inclusive, $range)` | Get descendant IDs recursively. |
| `fn::visible_geo_descendants($root, $inclusive, $range)` | Get descendants that have visible geometry. |
| `fn::ancestor($pe)` | Get the ancestor of a node. |
| `fn::collect_descendants_filter_inst(...)` | Filter descendants based on existing relations. |

**Usage in Rust**:
```rust
let sql = format!("fn::collect_descendant_ids_by_types({}, ['BOX'], true, '..')", pe_key);
```

## 4. Rust Integration Patterns

### 4.1 Basic Query Execution
```rust
use aios_core::{SUL_DB, SurrealQueryExt};

// Return a generic Vector of structs
let result: Vec<MyStruct> = SUL_DB.query_take(&sql, 0).await?;

// MyStruct definition
#[derive(Serialize, Deserialize, SurrealValue)]
struct MyStruct {
    id: RefnoEnum,
    name: String
}
```

### 4.2 Generic Recursive Helpers (Recommended)
Use `collect_descendant_with_expr` for flexibility.

```rust
// fetch ID only
let ids: Vec<RefnoEnum> = collect_descendant_with_expr(
    &[root_refno], 
    &["BOX", "CYLI"], // Filter types
    Some("1..5"),     // Depth range
    "VALUE id"        // Select expression
).await?;

// fetch Full Attributes
let attrs: Vec<NamedAttrMap> = collect_descendant_with_expr(
    &[root_refno], 
    &[], 
    None, 
    "VALUE id.refno.*" // Fetch nested object
).await?;
```

### 4.3 Batching Strategy (Performance)
**Never** loop over IDs to query individually. Use `array::map` inside the query.

**Bad (Loop in Rust)**:
```rust
for refno in refnos {
    SUL_DB.query("..."); // N network calls
}
```

**Good (Batch in SQL)**:
```rust
let sql = format!(
    "array::map([{}], |$id| fn::some_function($id))", 
    refnos.join(",")
);
// 1 network call
```

## 5. Migration / Version 3.0 Checklist
1.  **Strict Typing**: Ensure `SurrealValue` is used, not `serde_json::Value`.
2.  **Recursive Syntax**: deprecate old manual graph traversals in favor of `@.{..}` where possible.
3.  **Range Queries**: Use ID ranges `table:[min]..[max]` for scanning specific partitions (like `dbnum`).

## 6. Cheatsheet: Common Tasks

| Task | Pattern |
| :--- | :--- |
| **Get Children** | `collect_children_filter_ids(refno, &types)` |
| **Get Ancestors** | `query_filter_ancestors(refno, &types)` |
| **Get Visible Geometry** | `query_visible_geo_descendants(refno, ...)` |
| **Check Existence** | `WHERE count(SELECT ... LIMIT 1) > 0` (Don't use `count()` on full set) |
| **Deduplicate** | `array::distinct(...)` (No `SELECT DISTINCT`) |

## 7. File References
-   **Query Helpers**: `rs-core/src/rs_surreal/query_ext.rs`
-   **Graph Logic**: `rs-core/src/rs_surreal/graph.rs`
-   **DB Functions**: `rs-core/resource/surreal/common.surql`
