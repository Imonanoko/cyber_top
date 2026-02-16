# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust + Bevy 0.18 desktop game: spinning tops battle in a frictionless circular arena. Two game modes: PvP and PvAI.

## Build & Run

```bash
cargo build              # Build
cargo run                # Run the game
cargo build --release    # Release build
```

No test suite yet. No linter config beyond default `cargo` warnings.

## Key Documentation

- `docs/architecture.md` — Game flow, plugin structure, FixedUpdate pipeline, Bevy 0.18 API notes
- `docs/design.md` — Game design spec: damage model, weapon system, equipment, numerical safety, DB schema

## Architecture Principles

1. **Event-driven resolution**: collisions/attacks produce events (`GameEvent`), never directly modify HP.
2. **Data-driven specs**: parts/tops/weapons defined in `PartRegistry` with ID-based lookup.
3. **No DB IO in combat tick**: only read in-memory `EffectiveStats` and runtime state.
4. **All tunable params in `Tuning` resource**: loaded from `tuning.ron`, F5 hot-reload at runtime.

## State Machine

```
MainMenu → Selection → PickMap / PickTop → Aiming → Battle → GameOver → MainMenu
```

All `GamePhase` states defined in `src/game/components.rs`. Menu UI in `src/plugins/menu_plugin.rs`. Game systems in `src/plugins/game_plugin.rs`.

## FixedUpdate Pipeline (Battle phase only)

PhysicsSet → CollisionDetectSet → EventGenerateSet → HookProcessSet → EventApplySet → CleanupSet

All sets gated to `GamePhase::Battle` and chained in strict order. See `docs/architecture.md` for system-level detail.

## Bevy 0.18 API Gotchas

- `ChildSpawnerCommands` (not `ChildBuilder`) — the type for `with_children` closures
- `despawn()` is recursive by default — `despawn_recursive()` was removed
- `BorderRadius` is a field on `Node { border_radius: ..., ..default() }`, not a Component
- Bundle tuple max ~15 elements — nest inner tuples if needed
- Use `MessageWriter<T>` / `MessageReader<T>` for game events (not `Events<T>`)

## Entity Lifecycle

- Tag all game-session entities with `InGame` component
- `cleanup_game` on `OnEnter(MainMenu)` despawns all `InGame` entities
- Projectiles despawn on: hit, lifetime expiry, or leaving arena boundary

## Key Resources

| Resource | Location | Purpose |
|----------|----------|---------|
| `Tuning` | `src/config/tuning.rs` | All tunable constants |
| `PartRegistry` | `src/game/parts/registry.rs` | Data-driven part/top/weapon presets |
| `GameSelection` | `src/plugins/menu_plugin.rs` | Current mode, map, P1/P2 selections |
| `ProjectileAssets` | `src/game/components.rs` | Pre-built mesh/material for bullets |
