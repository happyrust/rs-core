# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is `aios_core`, a Rust-based plant engineering and design management system. The project handles complex industrial plant data, geometry, materials, and spatial relationships. It integrates with PDMS (Plant Design Management System) data and provides APIs for querying, managing, and analyzing plant design information.

## Build and Development Commands

### Basic Operations
```bash
# Build the project
cargo build

# Run tests
cargo test

# Build with verbose output
cargo build --verbose

# Run tests with verbose output
cargo test --verbose

# Build for release
cargo build --release
```

### Toolchain Information
- Uses Rust nightly toolchain (specified in `rust-toolchain.toml`)
- Edition: 2024
- Uses experimental features: `let_chains`, `trivial_bounds`, `result_flattening`

### Testing
```bash
# Run specific test modules
cargo test test_surreal
cargo test test_spatial
cargo test test_material

# Run tests for specific features
cargo test --features manifold
cargo test --features sql
```

## Architecture Overview

### Core Module Structure

**Database Layer (`rs_surreal/`)**
- Primary database integration with SurrealDB
- Connection management through `SUL_DB` and `SECOND_SUL_DB` global instances
- Query building, spatial operations, and data versioning
- Material list management with domain-specific modules (dq, gps, gy, nt, tf, tx, yk)

**Data Types (`types/`)**
- Core data structures: `RefNo`, `AttMap`, `AttVal`, `NamedAttMap`
- Database info structures and query SQL builders
- Hash utilities and reference number management

**Geometry and Spatial (`prim_geo/`, `spatial/`, `geometry/`)**
- Primitive geometric shapes: cylinders, spheres, boxes, pyramids, etc.
- Spatial calculations and acceleration trees
- Room and zone management with AABB (Axis-Aligned Bounding Box) trees

**Materials and Plant Data (`material/`, `room/`)**
- Material classification and management (dq, gps, gy, nt, sb, tf, tx, yk systems)
- Room calculations, hierarchy, and spatial relationships
- HVAC and piping material calculations

**Configuration and Options**
- Database configuration through `DbOption.toml` files
- Multiple environment support (ABA, AMS variants)
- Project-specific settings and connection strings

### Key Design Patterns

**Database Abstraction**
- Global database connections (`SUL_DB`, `SECOND_SUL_DB`)
- Async initialization functions (`init_surreal()`, `init_test_surreal()`)
- Configuration-driven database setup

**Type System**
- Heavy use of reference numbers (`RefNo`, `RefU64`) for entity identification
- Attribute maps for flexible property storage
- Strong typing with custom derive macros

**Modular Architecture**
- Feature-gated compilation (occ, manifold, sql, render)
- Domain-specific modules for different plant systems
- Clear separation between data types, operations, and storage

## Configuration Files

- `DbOption.toml` - Primary database configuration
- `DbOption_ABA.toml`, `DbOption_AMS.toml` - Environment-specific configs
- `all_attr_info.json` - PDMS database metadata
- Material configuration Excel files in `src/rs_surreal/material_list/tf/`

## Version Control API

The project includes a version management system:

```rust
// Query all history for a session number
aios_core::query_ses_history(sesno: i32) -> Vec<HisRefno>

// Query history for a specific reference number
aios_core::query_history_data(refno: Refno) -> Vec<HisRefno>

// Get differences between two session numbers
aios_core::diff_sesno(refno: Refno, sesno1: i32, sesno2: i32) -> Vec<Diff>
```

## Important Dependencies

- **SurrealDB**: Primary database (custom fork from gitee.com/happydpc/surrealdb)
- **Bevy**: Math and transform utilities
- **Glam**: Vector mathematics
- **Parry**: Geometric collision detection
- **Nalgebra**: Linear algebra operations
- **Manifold**: 3D geometry operations (feature-gated)

## Development Notes

- The project uses experimental Rust features - ensure nightly toolchain
- Database connections require valid `DbOption.toml` configuration
- Material list generation involves complex SurrealQL scripts in `src/rs_surreal/material_list/`
- Spatial computations are performance-critical and use acceleration structures
- Test modules are organized by functional area under `src/test/`

## Working with the Codebase

When making changes:
1. Understand the modular structure - changes often span multiple modules
2. Database queries use SurrealQL - see examples in `material_list/` subdirectories
3. Geometric operations require understanding of the coordinate systems used
4. Material calculations follow domain-specific business rules
5. Always test with various `DbOption.toml` configurations