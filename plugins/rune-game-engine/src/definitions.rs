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
        ToolDefinition {
            name: "compile_declarative_bundle".to_string(),
            description: "Compiles a declarative JSON specification into a complete single-file HTML/WebGL artifact server-side. Eliminates token streaming by pre-validating engine templates and assembling artifacts deterministically.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "engine": { "type": "string", "description": "Rendering backend (threejs, canvas2d)" },
                    "theme": { "type": "string", "description": "Visual theme name (ghibli_coastal, cyberpunk, minimal)" },
                    "pipeline": {
                        "type": "array",
                        "description": "Pipeline steps to execute (spline, biome, kinematics, audio, rig)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "description": "Step type identifier" },
                                "params": { "type": "object", "description": "Step configuration" }
                            },
                            "required": ["type"]
                        }
                    },
                    "assets": { "type": "object", "description": "Additional assets to embed" },
                    "ui_config": { "type": "object", "description": "UI layout configuration" }
                },
                "required": ["engine", "theme", "pipeline"]
            }),
        },
        ToolDefinition {
            name: "batch_pipeline_executor".to_string(),
            description: "Executes multi-stage computational graphs in a single server-side cycle using dependency DAGs. Reduces tool round-trips from 15 to 1.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "steps": {
                        "type": "array",
                        "description": "Pipeline steps to execute",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "description": "Step type" },
                                "params": { "type": "object", "description": "Step parameters" }
                            },
                            "required": ["type"]
                        }
                    },
                    "input_state": { "type": "object", "description": "Initial state for pipeline" },
                    "parallel": { "type": "boolean", "default": false, "description": "Execute steps in parallel" }
                },
                "required": ["steps", "input_state"]
            }),
        },
        ToolDefinition {
            name: "patch_ast_node".to_string(),
            description: "Performs AST-aware source code mutations targeting functions or object keys by identifier. Eliminates regex/string-matching errors like CRLF/LF mismatches.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to file to patch" },
                    "node_type": { "type": "string", "description": "AST node type (VariableDeclaration, ObjectProperty, etc.)" },
                    "node_name": { "type": "string", "description": "Identifier/name to target" },
                    "operation": { "type": "string", "enum": ["set", "update", "delete", "add"], "description": "Mutation operation" },
                    "new_value": { "type": "object", "description": "New value to set" },
                    "context": { "type": "object", "description": "Optional context for mutation" }
                },
                "required": ["file_path", "node_type", "node_name", "operation"]
            }),
        },
        ToolDefinition {
            name: "validate_headless_runtime".to_string(),
            description: "Executes code inside a headless sandbox (Playwright/Puppeteer) to output syntax errors, unhandled exceptions, and runtime FPS. Eliminates manual inspection scripts.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "html_bundle": { "type": "string", "description": "HTML bundle to validate" },
                    "test_cases": { "type": "array", "items": { "type": "string" }, "description": "Test case names to run" },
                    "timeout_ms": { "type": "integer", "default": 30000, "description": "Execution timeout" },
                    "headless": { "type": "boolean", "default": true, "description": "Run in headless mode" }
                },
                "required": ["html_bundle"]
            }),
        },
        ToolDefinition {
            name: "cache_template_registry".to_string(),
            description: "Caches standard CDN bundles, UI boilerplate CSS, and math libraries locally so the model only configures deltas. Saves ~10,000 repetitive boilerplate tokens per project.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "template_name": { "type": "string", "description": "Name of template to cache" },
                    "template_type": { "type": "string", "enum": ["html_boilerplate", "css_theme", "math_library", "webgl_stub"], "description": "Template category" },
                    "custom_config": { "type": "object", "description": "Custom configuration for template" }
                },
                "required": ["template_name", "template_type"]
            }),
        },
        // --- Gameplay & Experience Enhancement Tools ---
        ToolDefinition {
            name: "simulate_virtual_playtest".to_string(),
            description: "Executes automated player behaviors in a headless browser to collect quantitative telemetry on comfort, camera jerk, input lag, and frustration events. Detects blind design issues before manual QA.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "html_bundle": { "type": "string", "description": "HTML bundle to test" },
                    "input_script": {
                        "type": "array",
                        "description": "Automated player input sequence",
                        "items": {
                            "type": "object",
                            "properties": {
                                "time_ms": { "type": "integer", "description": "Timestamp in milliseconds" },
                                "keys_down": { "type": "array", "items": { "type": "string" }, "description": "Keys pressed" },
                                "keys_up": { "type": "array", "items": { "type": "string" }, "description": "Keys released" }
                            },
                            "required": ["time_ms"]
                        }
                    },
                    "metrics_to_track": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["g_forces", "comfort", "fps", "camera_jerk", "input_lag"] },
                        "description": "Metrics to collect during simulation"
                    }
                },
                "required": ["html_bundle"]
            }),
        },
        ToolDefinition {
            name: "configure_camera_controller".to_string(),
            description: "Generates smooth cinematic camera rigs with spring damping, dynamic FOV expansion, spline look-ahead, and frustum bounds. Eliminates stiff or disorienting camera behavior.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "camera_mode": { "type": "string", "description": "Camera mode", "enum": ["follow_rail", "spring_arm", "cockpit", "isometric"] },
                    "target_entity_id": { "type": "string", "description": "Target entity to follow" },
                    "look_ahead": {
                        "type": "object",
                        "description": "Look-ahead configuration for spline following",
                        "properties": {
                            "enabled": { "type": "boolean", "description": "Enable look-ahead", "default": true },
                            "spline_lead_t": { "type": "number", "description": "Look-ahead distance along spline", "minimum": 0.0, "maximum": 0.5 }
                        },
                        "required": ["enabled", "spline_lead_t"]
                    },
                    "dynamic_fov": {
                        "type": "object",
                        "description": "Dynamic field of view configuration",
                        "properties": {
                            "base_fov": { "type": "number", "description": "Base field of view", "minimum": 30, "maximum": 120 },
                            "max_fov": { "type": "number", "description": "Maximum FOV at high speed", "minimum": 60, "maximum": 120 },
                            "speed_scaler": { "type": "number", "description": "FOV expansion factor", "minimum": 0.0, "maximum": 1.0 }
                        },
                        "required": ["base_fov", "max_fov", "speed_scaler"]
                    },
                    "damping": {
                        "type": "object",
                        "description": "Camera damping configuration",
                        "properties": {
                            "position_lag": { "type": "number", "description": "Position interpolation lag", "minimum": 0.0, "maximum": 1.0 },
                            "rotation_lag": { "type": "number", "description": "Rotation interpolation lag", "minimum": 0.0, "maximum": 1.0 }
                        },
                        "required": ["position_lag", "rotation_lag"]
                    }
                },
                "required": ["camera_mode", "damping", "dynamic_fov", "look_ahead"]
            }),
        },
        ToolDefinition {
            name: "synthesize_game_feel_system".to_string(),
            description: "Transforms sterile movement into tactile game feel by orchestrating screen shake, particles, audio ducking, chromatic aberration, chassis sway, and speed lines. Adds weight, momentum, and impact feedback.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "feedback_channels": {
                        "type": "array",
                        "description": "Feedback channels to activate",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "enum": ["screen_shake", "particles", "audio_ducking", "chromatic_aberration", "chassis_sway", "wind_particles"] },
                                "intensity_scale": { "type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.5 }
                            },
                            "required": ["name"]
                        }
                    },
                    "event_mappings": {
                        "type": "array",
                        "description": "Event mappings for feedback triggers",
                        "items": {
                            "type": "object",
                            "properties": {
                                "trigger": { "type": "string", "description": "Event trigger (cornering, hard_brake, impact, drift)" },
                                "intensity_metric": { "type": "string", "description": "Metric that drives intensity" },
                                "action": { "type": "string", "description": "Feedback action to execute" }
                            },
                            "required": ["trigger", "action"]
                        }
                    }
                },
                "required": ["feedback_channels", "event_mappings"]
            }),
        },
        ToolDefinition {
            name: "tune_gameplay_parameters".to_string(),
            description: "Generates lightweight on-screen slider panels (Tweakpane/Dat.GUI) or headless config hooks to expose hardcoded values (braking power, comfort decay, friction) for runtime tuning without recompilation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "harness_type": { "type": "string", "enum": ["tweakpane", "dat_gui", "headless_config"], "description": "UI harness type" },
                    "tunable_registry": {
                        "type": "array",
                        "description": "List of tunable parameters",
                        "items": {
                            "type": "object",
                            "properties": {
                                "key": { "type": "string", "description": "Parameter key" },
                                "path": { "type": "string", "description": "Object path to parameter" },
                                "type": { "type": "string", "enum": ["number", "boolean", "select"], "description": "Parameter type" },
                                "min": { "type": "number", "description": "Minimum value" },
                                "max": { "type": "number", "description": "Maximum value" },
                                "step": { "type": "number", "description": "Step size" },
                                "default": { "type": "number", "description": "Default value" }
                            },
                            "required": ["key", "path", "type", "default"]
                        }
                    },
                    "preset_profiles": { "type": "object", "description": "Named presets for different tuning modes" }
                },
                "required": ["harness_type", "tunable_registry"]
            }),
        },
        ToolDefinition {
            name: "balance_mechanic_economy".to_string(),
            description: "Prevents abrupt mechanic triggers by applying sigmoid smoothing, grace periods (coyote time), and fair decay windows. Balances streak rewards, tip bonuses, and comfort penalties with mathematical elegance.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_variables": {
                        "type": "array",
                        "description": "Input variables for the mechanic",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "Variable name" },
                                "domain": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2, "description": "[min, max] domain" }
                            },
                            "required": ["name", "domain"]
                        }
                    },
                    "target_metric": { "type": "string", "enum": ["comfort_score", "tip_reward", "streak_decay"], "description": "Target metric to balance" },
                    "curve_type": { "type": "string", "enum": ["exponential", "logarithmic", "sigmoid", "piecewise"], "description": "Curve shape for smoothing" },
                    "forgiveness_window_sec": { "type": "number", "minimum": 0.0, "maximum": 10.0, "default": 0.5, "description": "Grace period in seconds" }
                },
                "required": ["input_variables", "target_metric", "curve_type"]
            }),
        },
        ToolDefinition {
            name: "audit_shader_and_material_pipeline".to_string(),
            description: "Validates shader uniforms, lighting setups, material properties, and fog configuration before runtime. Prevents black-screen bugs and Three.js refreshFogUniforms crashes from missing uniforms.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "scene_materials": {
                        "type": "array",
                        "description": "Scene materials to validate",
                        "items": {
                            "type": "object",
                            "properties": {
                                "material_id": { "type": "string", "description": "Material identifier" },
                                "type": { "type": "string", "description": "Material type (MeshBasicMaterial, MeshStandardMaterial, etc.)" },
                                "shaders": { "type": "object", "description": "Custom shader definitions" },
                                "uniforms": { "type": "object", "description": "Required uniforms" }
                            },
                            "required": ["material_id", "type"]
                        }
                    },
                    "environment_features": {
                        "type": "object",
                        "description": "Environment rendering features",
                        "properties": {
                            "has_fog": { "type": "boolean", "description": "Whether environment has fog", "default": false },
                            "tonemapping": { "type": "string", "description": "Tone mapping algorithm", "enum": ["linear", "reinhard", "acg"] }
                        }
                    }
                },
                "required": ["scene_materials", "environment_features"]
            }),
        },
    ]
}
