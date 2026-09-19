# rune-game-engine

General-purpose 3D Web Game & Simulation Engine MCP Plugin for Rune.

## Overview

`rune-game-engine` provides foundational primitives for generating web-based 3D games, transit simulations, and interactive experiences directly inside an AI model interaction loop. It runs as a sandboxed WASM plugin (`wasm32-wasip1`) and produces complete, zero-build-step HTML/WebGL artifacts.

## Capabilities

- **Spatial Math & Splines**: Smooth Catmull-Rom/Bezier curve generation, Frenet-Serret reference frames, curvature metrics, and extrusion buffers (`generate_path_network`).
- **Procedural Graphics**: Biome anchoring, scatter manifests, lighting, and gradient skies (`generate_procedural_biome`).
- **Rigging & Kinematics**: Hierarchical chassis assembly, interactive door/wheel hooks, camera sockets (`assemble_modular_rig`), and physics simulation with grade, drag, and centrifugal roll (`simulate_path_kinematics`).
- **Gameplay & Audio**: Ride comfort/jerk evaluation (`evaluate_dynamic_comfort`), zone lifecycle triggers (`manage_zone_lifecycle`), and procedural Web Audio API synthesis graphs (`synthesize_audio_graph`).
- **Build & Packaging**: Single-file HTML bundle generation (`bundle_webgl_container`) and pre-flight performance audit (`inspect_webgl_performance`).

## Tools

### generate_path_network

Calculates smooth 3D splines with Frenet-Serret reference frames.

**Input:**
```json
{
  "nodes": [
    { "id": "n1", "position": [0, 0, 0] },
    { "id": "n2", "position": [10, 5, 20] },
    { "id": "n3", "position": [20, 0, 50] }
  ],
  "curve_type": "catmull_rom",
  "cross_section": "rail"
}
```

**Output:** Path points, frames (tangents, normals, binormals), and extrusion mesh definition.

### generate_procedural_biome

Generates procedural scene geometry and lighting configurations.

**Input:**
```json
{
  "terrain_type": "floating_islands",
  "palette": {
    "sky_gradient": ["#87CEEB", "#2C3E50"],
    "ambient": "#FFF5E6",
    "surface_colors": ["#8B4513", "#228B22"]
  },
  "boundary_box": [[0, -20, 0], [100, 20, 100]]
}
```

**Output:** Scene manifest with assets, lighting config, and skybox definition.

### assemble_modular_rig

Assembles modular vehicle hierarchies with interactive components.

**Input:**
```json
{
  "base_chassis": {
    "dimensions": [4.0, 2.5, 2.0],
    "pivot_offset": [0, 0, 0]
  },
  "sockets": [
    { "socket_id": "wheel_fl", "transform": [1.0, 0, -1.0] }
  ]
}
```

**Output:** Hierarchy graph, interactive parts, and camera anchor positions.

### simulate_path_kinematics

Advances vehicle dynamics along a spline path.

**Input:**
```json
{
  "transform_state": {
    "t_normalized": 0.25,
    "velocity": 12.0
  },
  "control_intent": {
    "throttle": 1.0,
    "brake": 0.0
  },
  "track_metrics": {
    "curvature": 0.05,
    "grade_angle": 0.0
  },
  "delta_time": 0.0166
}
```

**Output:** Next state (t, velocity, acceleration), body roll, and G-force vector.

### evaluate_dynamic_comfort

Evaluates ride quality and applies comfort penalties.

**Input:**
```json
{
  "g_force_history": [
    { "lateral_g": 0.55, "vertical_g": 1.0, "jerk": 3.2 }
  ],
  "current_comfort_score": 90.0,
  "streak_status": { "active": true, "count": 10 }
}
```

**Output:** Updated comfort score, penalty status, and streak information.

### manage_zone_lifecycle

Handles station/checkpoint transitions and triggers.

**Input:**
```json
{
  "zone_type": "station",
  "trigger_event": "dock",
  "entity_state": {},
  "zone_context": { "bonus_values": 100.0 }
}
```

**Output:** Lifecycle phase, UI notifications, and score delta.

### synthesize_audio_graph

Generates Web Audio API code for procedural sound effects.

**Input:**
```json
{
  "audio_preset": "engine_drone",
  "driver_parameters": {}
}
```

**Output:** Web Audio node graph code and routing configuration.

### bundle_webgl_container

Creates single-file HTML bundles with WebGL rendering.

**Input:**
```json
{
  "renderer": "threejs",
  "engine_scripts": ["console.log('Ready');"],
  "ui_layout": {
    "hud_anchors": { "top_left": "status" }
  }
}
```

**Output:** Complete HTML document with embedded scripts.

### inspect_webgl_performance

Audits WebGL bundle performance.

**Input:**
```json
{
  "html_bundle": "<html>...</html>",
  "target_fps": 60,
  "draw_call_limit": 150
}
```

**Output:** Performance metrics, warnings, and pass/fail status.

## Testing & Building

```bash
# Run unit and contract tests locally (no WASM toolchain required)
cargo test -p rune-game-engine

# Build WASM binary via xtask orchestrator
cargo xtask build rune-game-engine --wasm-only
```

## Architecture

This plugin is **WASM-only** (pure compute) with no native sidecar requirements. All operations are implemented using:
- Standard JavaScript math for spline calculations
- Procedural JSON generation for scene data
- Web Audio API code synthesis
- Template-based HTML bundling

## License

MIT
