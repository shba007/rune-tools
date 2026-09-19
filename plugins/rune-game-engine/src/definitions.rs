use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "generate_path_network".to_string(),
            description: "Calculates smooth 3D splines (Catmull-Rom/Bezier), Frenet-Serret reference frames, curvature metrics, and rail/road extrusion geometry.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "nodes": {
                        "type": "array",
                        "description": "Control nodes with 3D positions and optional banking angles",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string", "description": "Node identifier" },
                                "position": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "3D position [x, y, z]" },
                                "banking": { "type": "number", "description": "Banking angle" },
                                "tangent": { "type": "array", "items": { "type": "number" }, "description": "Tangent vector [x, y, z]" }
                            },
                            "required": ["id", "position"]
                        }
                    },
                    "curve_type": { "type": "string", "enum": ["catmull_rom", "bezier", "bspline"], "default": "catmull_rom", "description": "Type of spline interpolation" },
                    "closed": { "type": "boolean", "default": false, "description": "Whether the path forms a closed loop" },
                    "cross_section": { "type": "string", "enum": ["rail", "pipe", "road", "custom"], "default": "rail", "description": "Cross-section profile type" },
                    "profile_options": {
                        "type": "object",
                        "description": "Profile configuration options",
                        "properties": {
                            "gauge_width": { "type": "number", "description": "Track gauge width" },
                            "tie_spacing": { "type": "number", "description": "Spacing between ties" },
                            "pylon_interval": { "type": "number", "description": "Interval between pylons" }
                        }
                    }
                },
                "required": ["nodes"]
            }),
        },
        ToolDefinition {
            name: "generate_procedural_biome".to_string(),
            description: "Generates procedural scene geometry, terrain anchors, scattering manifests, and lighting/skybox definitions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "terrain_type": { "type": "string", "enum": ["floating_islands", "heightmap", "archipelago"], "description": "Type of terrain to generate" },
                    "palette": {
                        "type": "object",
                        "description": "Visual palette configuration",
                        "properties": {
                            "sky_gradient": { "type": "array", "items": { "type": "string" }, "minItems": 2, "maxItems": 2, "description": "Sky gradient colors [start, end]" },
                            "ambient": { "type": "string", "description": "Ambient light color" },
                            "surface_colors": { "type": "array", "items": { "type": "string" }, "description": "Surface material colors" }
                        },
                        "required": ["sky_gradient", "ambient", "surface_colors"]
                    },
                    "feature_scatter": {
                        "type": "array",
                        "description": "Scattered features in the biome",
                        "items": {
                            "type": "object",
                            "properties": {
                                "asset_type": { "type": "string", "description": "Type of asset" },
                                "density": { "type": "number", "description": "Spawn density" },
                                "cluster_rule": { "type": "string", "description": "Clustering rule" }
                            },
                            "required": ["asset_type", "density"]
                        }
                    },
                    "boundary_box": {
                        "type": "array",
                        "description": "3D boundary box [[min_x, min_y, min_z], [max_x, max_y, max_z]]",
                        "items": { "type": "array", "items": { "type": "number" } }
                    }
                },
                "required": ["terrain_type", "palette", "boundary_box"]
            }),
        },
        ToolDefinition {
            name: "assemble_modular_rig".to_string(),
            description: "Assembles modular vehicle or avatar hierarchies, attachment sockets, and interactive sub-components with camera anchors.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "base_chassis": {
                        "type": "object",
                        "description": "Base chassis definition",
                        "properties": {
                            "dimensions": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "Chassis dimensions [x, y, z]" },
                            "pivot_offset": { "type": "array", "items": { "type": "number" }, "description": "Pivot offset [x, y, z]" }
                        },
                        "required": ["dimensions"]
                    },
                    "sockets": {
                        "type": "array",
                        "description": "Attachment sockets",
                        "items": {
                            "type": "object",
                            "properties": {
                                "socket_id": { "type": "string", "description": "Unique socket identifier" },
                                "transform": { "type": "array", "items": { "type": "number" }, "description": "Socket transform [x, y, z]" }
                            },
                            "required": ["socket_id", "transform"]
                        }
                    },
                    "attachments": { "type": "array", "items": { "type": "object" }, "description": "Attached components" },
                    "interior_occupants": { "type": "array", "items": { "type": "object" }, "description": "Interior occupants" }
                },
                "required": ["base_chassis"]
            }),
        },
        ToolDefinition {
            name: "simulate_path_kinematics".to_string(),
            description: "Advances 1D/spline-constrained dynamics with throttle, braking, track grade, centrifugal roll, and external forces.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "transform_state": {
                        "type": "object",
                        "description": "Current transform state",
                        "properties": {
                            "t_normalized": { "type": "number", "description": "Normalized progress along path (0-1)" },
                            "velocity": { "type": "number", "description": "Current velocity" },
                            "acceleration": { "type": "number", "description": "Current acceleration" }
                        },
                        "required": ["t_normalized", "velocity"]
                    },
                    "control_intent": {
                        "type": "object",
                        "description": "Driver control inputs",
                        "properties": {
                            "throttle": { "type": "number", "description": "Throttle input (0-1)" },
                            "brake": { "type": "number", "description": "Brake input (0-1)" }
                        },
                        "required": ["throttle", "brake"]
                    },
                    "track_metrics": {
                        "type": "object",
                        "description": "Track geometry metrics",
                        "properties": {
                            "curvature": { "type": "number", "description": "Track curvature" },
                            "grade_angle": { "type": "number", "description": "Track grade angle" }
                        }
                    },
                    "external_forces": {
                        "type": "object",
                        "description": "External forces acting on vehicle",
                        "properties": {
                            "vector": { "type": "array", "items": { "type": "number" }, "description": "Force vector [x, y, z]" },
                            "drag_coeff": { "type": "number", "description": "Drag coefficient" }
                        }
                    },
                    "vehicle_specs": {
                        "type": "object",
                        "description": "Vehicle specifications",
                        "properties": {
                            "max_speed": { "type": "number", "description": "Maximum speed" },
                            "power_accel": { "type": "number", "description": "Power acceleration" },
                            "brake_decel": { "type": "number", "description": "Braking deceleration" },
                            "suspension_roll_factor": { "type": "number", "description": "Suspension roll factor" }
                        }
                    },
                    "delta_time": { "type": "number", "default": 0.0166, "description": "Time step in seconds" }
                },
                "required": ["transform_state", "control_intent"]
            }),
        },
        ToolDefinition {
            name: "evaluate_dynamic_comfort".to_string(),
            description: "Evaluates passenger/cargo ride quality, G-force tolerances, jerk penalties, and streak status.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "g_force_history": {
                        "type": "array",
                        "description": "History of G-force samples",
                        "items": {
                            "type": "object",
                            "properties": {
                                "lateral_g": { "type": "number", "description": "Lateral G-force" },
                                "vertical_g": { "type": "number", "description": "Vertical G-force" },
                                "jerk": { "type": "number", "description": "Jerk value" }
                            },
                            "required": ["lateral_g", "vertical_g", "jerk"]
                        }
                    },
                    "current_comfort_score": { "type": "number", "description": "Current comfort score (0-100)" },
                    "thresholds": {
                        "type": "object",
                        "description": "Comfort thresholds",
                        "properties": {
                            "max_comfortable_g": { "type": "number", "description": "Maximum comfortable G-force" },
                            "jerk_penalty_rate": { "type": "number", "description": "Jerk penalty multiplier" }
                        }
                    },
                    "streak_status": {
                        "type": "object",
                        "description": "Current streak status",
                        "properties": {
                            "active": { "type": "boolean", "description": "Whether a streak is active" },
                            "count": { "type": "integer", "description": "Current streak count" }
                        },
                        "required": ["active", "count"]
                    }
                },
                "required": ["g_force_history", "current_comfort_score", "streak_status"]
            }),
        },
        ToolDefinition {
            name: "manage_zone_lifecycle".to_string(),
            description: "Handles station, hazard, or checkpoint lifecycle transitions, door mechanics, bonuses, and HUD notifications.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "zone_type": { "type": "string", "enum": ["station", "checkpoint", "hazard", "workshop"], "description": "Type of zone" },
                    "trigger_event": { "type": "string", "enum": ["enter", "dock", "exit"], "description": "Trigger event" },
                    "entity_state": { "type": "object", "description": "Current entity state" },
                    "zone_context": {
                        "type": "object",
                        "description": "Zone context information",
                        "properties": {
                            "queue_count": { "type": "integer", "description": "Queue count" },
                            "subtitles": { "type": "string", "description": "Subtitle text" },
                            "bonus_values": { "type": "number", "description": "Bonus values" }
                        }
                    }
                },
                "required": ["zone_type", "trigger_event", "entity_state"]
            }),
        },
        ToolDefinition {
            name: "synthesize_audio_graph".to_string(),
            description: "Synthesizes modular Web Audio API node graphs for zero-asset procedural sound effects (rail clatter, wind, bells, engines).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "audio_preset": { "type": "string", "enum": ["engine_drone", "friction_surface", "chime", "environmental_wind"], "description": "Audio preset type" },
                    "driver_parameters": { "type": "object", "description": "Driver-specific parameters" }
                },
                "required": ["audio_preset"]
            }),
        },
        ToolDefinition {
            name: "bundle_webgl_container".to_string(),
            description: "Bundles simulation logic, graphics, audio, and HUD into a single zero-dependency standalone HTML file.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "renderer": { "type": "string", "enum": ["threejs", "canvas2d"], "default": "threejs", "description": "Rendering backend" },
                    "engine_scripts": { "type": "array", "items": { "type": "string" }, "description": "Engine script code snippets" },
                    "ui_layout": {
                        "type": "object",
                        "description": "UI layout configuration",
                        "properties": {
                            "hud_anchors": { "type": "object", "description": "HUD element positions" },
                            "theme_css": { "type": "string", "description": "Custom CSS theme" },
                            "control_mode": { "type": "string", "description": "Control input mode" }
                        },
                        "required": ["hud_anchors"]
                    },
                    "embedded_assets": { "type": "object", "description": "Embedded assets" }
                },
                "required": ["engine_scripts", "ui_layout"]
            }),
        },
        ToolDefinition {
            name: "inspect_webgl_performance".to_string(),
            description: "Audits a WebGL HTML bundle against target framerate, draw call budgets, memory estimates, and platform constraints.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "html_bundle": { "type": "string", "description": "HTML bundle content to inspect" },
                    "target_fps": { "type": "integer", "default": 60, "description": "Target framerate" },
                    "draw_call_limit": { "type": "integer", "default": 150, "description": "Maximum draw calls" },
                    "simulated_devices": { "type": "array", "items": { "type": "string" }, "description": "Devices to simulate" }
                },
                "required": ["html_bundle"]
            }),
        },
    ]
}
