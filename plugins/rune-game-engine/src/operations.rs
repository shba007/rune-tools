use crate::types::*;
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    match request.name.as_str() {
        "generate_path_network" => {
            let params: GeneratePathNetworkParams = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid parameters for generate_path_network: {}", e))?;
            op_generate_path_network(params)
        }
        "generate_procedural_biome" => {
            let params: GenerateProceduralBiomeParams = serde_json::from_value(request.arguments)
                .map_err(|e| {
                format!("Invalid parameters for generate_procedural_biome: {}", e)
            })?;
            op_generate_procedural_biome(params)
        }
        "assemble_modular_rig" => {
            let params: AssembleModularRigParams = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid parameters for assemble_modular_rig: {}", e))?;
            op_assemble_modular_rig(params)
        }
        "simulate_path_kinematics" => {
            let params: SimulatePathKinematicsParams = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid parameters for simulate_path_kinematics: {}", e))?;
            op_simulate_path_kinematics(params)
        }
        "evaluate_dynamic_comfort" => {
            let params: EvaluateDynamicComfortParams = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid parameters for evaluate_dynamic_comfort: {}", e))?;
            op_evaluate_dynamic_comfort(params)
        }
        "manage_zone_lifecycle" => {
            let params: ManageZoneLifecycleParams = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid parameters for manage_zone_lifecycle: {}", e))?;
            op_manage_zone_lifecycle(params)
        }
        "synthesize_audio_graph" => {
            let params: SynthesizeAudioGraphParams = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid parameters for synthesize_audio_graph: {}", e))?;
            op_synthesize_audio_graph(params)
        }
        "bundle_webgl_container" => {
            let params: BundleWebglContainerParams = serde_json::from_value(request.arguments)
                .map_err(|e| format!("Invalid parameters for bundle_webgl_container: {}", e))?;
            op_bundle_webgl_container(params)
        }
        "inspect_webgl_performance" => {
            let params: InspectWebglPerformanceParams = serde_json::from_value(request.arguments)
                .map_err(|e| {
                format!("Invalid parameters for inspect_webgl_performance: {}", e)
            })?;
            op_inspect_webgl_performance(params)
        }
        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}

// --- Tool Implementations ---

fn op_generate_path_network(params: GeneratePathNetworkParams) -> Result<Value, String> {
    if params.nodes.len() < 2 {
        return Err("generate_path_network requires at least 2 control nodes".to_string());
    }

    let samples_per_segment = 10;
    let mut sampled_points: Vec<[f64; 3]> = Vec::new();
    let mut frames = Vec::new();
    let mut lookup_table = Vec::new();

    let n = params.nodes.len();
    let segments = if params.closed { n } else { n - 1 };
    let mut total_distance = 0.0;

    for i in 0..segments {
        let p0 = if i == 0 {
            if params.closed {
                params.nodes[n - 1].position
            } else {
                params.nodes[0].position
            }
        } else {
            params.nodes[i - 1].position
        };
        let p1 = params.nodes[i % n].position;
        let p2 = params.nodes[(i + 1) % n].position;
        let p3 = if (i + 2) < n {
            params.nodes[i + 2].position
        } else if params.closed {
            params.nodes[(i + 2) % n].position
        } else {
            p2
        };

        for step in 0..samples_per_segment {
            let t = step as f64 / samples_per_segment as f64;
            // Standard Centripetal/Uniform Catmull-Rom formulation
            let t2 = t * t;
            let t3 = t2 * t;

            let x = 0.5
                * ((2.0 * p1[0])
                    + (-p0[0] + p2[0]) * t
                    + (2.0 * p0[0] - 5.0 * p1[0] + 4.0 * p2[0] - p3[0]) * t2
                    + (-p0[0] + 3.0 * p1[0] - 3.0 * p2[0] + p3[0]) * t3);
            let y = 0.5
                * ((2.0 * p1[1])
                    + (-p0[1] + p2[1]) * t
                    + (2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1]) * t2
                    + (-p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1]) * t3);
            let z = 0.5
                * ((2.0 * p1[2])
                    + (-p0[2] + p2[2]) * t
                    + (2.0 * p0[2] - 5.0 * p1[2] + 4.0 * p2[2] - p3[2]) * t2
                    + (-p0[2] + 3.0 * p1[2] - 3.0 * p2[2] + p3[2]) * t3);

            let pt = [x, y, z];
            if let Some(prev) = sampled_points.last() {
                let d =
                    ((x - prev[0]).powi(2) + (y - prev[1]).powi(2) + (z - prev[2]).powi(2)).sqrt();
                total_distance += d;
            }
            sampled_points.push(pt);

            // Compute tangent approximation
            let tx = 0.5
                * ((-p0[0] + p2[0])
                    + 2.0 * (2.0 * p0[0] - 5.0 * p1[0] + 4.0 * p2[0] - p3[0]) * t
                    + 3.0 * (-p0[0] + 3.0 * p1[0] - 3.0 * p2[0] + p3[0]) * t2);
            let ty = 0.5
                * ((-p0[1] + p2[1])
                    + 2.0 * (2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1]) * t
                    + 3.0 * (-p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1]) * t2);
            let tz = 0.5
                * ((-p0[2] + p2[2])
                    + 2.0 * (2.0 * p0[2] - 5.0 * p1[2] + 4.0 * p2[2] - p3[2]) * t
                    + 3.0 * (-p0[2] + 3.0 * p1[2] - 3.0 * p2[2] + p3[2]) * t2);
            let t_len = (tx * tx + ty * ty + tz * tz).sqrt().max(1e-6);

            let tangent = [tx / t_len, ty / t_len, tz / t_len];
            let normal = [-tangent[2], 0.0, tangent[0]]; // simplified horizontal normal
            let binormal = [
                tangent[1] * normal[2] - tangent[2] * normal[1],
                tangent[2] * normal[0] - tangent[0] * normal[2],
                tangent[0] * normal[1] - tangent[1] * normal[0],
            ];

            frames.push(json!({
                "tangent": tangent,
                "normal": normal,
                "binormal": binormal
            }));

            let norm_t = (i as f64 + t) / (segments as f64);
            lookup_table.push(json!({
                "t": norm_t,
                "distance": total_distance,
                "curvature": (1.0 / t_len).min(0.2)
            }));
        }
    }

    let opts = params.profile_options.unwrap_or(ProfileOptions {
        gauge_width: 2.0,
        tie_spacing: 1.5,
        pylon_interval: 30.0,
    });

    Ok(json!({
        "status": "success",
        "total_distance": total_distance,
        "sample_count": sampled_points.len(),
        "path_points": sampled_points,
        "frames": frames,
        "sample_lookup_table": lookup_table,
        "extrusion_mesh_def": {
            "cross_section": params.cross_section,
            "gauge_width": opts.gauge_width,
            "tie_spacing": opts.tie_spacing,
            "pylon_interval": opts.pylon_interval,
            "vertex_budget": sampled_points.len() * 8
        }
    }))
}

fn op_generate_procedural_biome(params: GenerateProceduralBiomeParams) -> Result<Value, String> {
    let mut manifest = Vec::new();
    let min_b = params.boundary_box[0];
    let max_b = params.boundary_box[1];

    let span_x = (max_b[0] - min_b[0]).abs();
    let span_z = (max_b[2] - min_b[2]).abs();

    for scatter in &params.feature_scatter {
        let count = ((span_x * span_z / 10000.0) * scatter.density).clamp(1.0, 50.0) as usize;
        for i in 0..count {
            let offset_x = min_b[0] + (i as f64 * 37.0 % span_x);
            let offset_z = min_b[2] + (i as f64 * 59.0 % span_z);
            let y_level = min_b[1] + ((i as f64 * 13.0) % 20.0);

            manifest.push(json!({
                "id": format!("{}_{}", scatter.asset_type, i),
                "primitive_type": scatter.asset_type,
                "transform": {
                    "position": [offset_x, y_level, offset_z],
                    "rotation": [0.0, (i as f64 * 0.7) % 6.28, 0.0],
                    "scale": [1.0, 1.0, 1.0]
                },
                "material_def": {
                    "color": params.palette.surface_colors[i % params.palette.surface_colors.len()]
                }
            }));
        }
    }

    Ok(json!({
        "status": "success",
        "terrain_type": params.terrain_type,
        "scene_manifest": manifest,
        "lighting_config": {
            "ambient": { "color": params.palette.ambient, "intensity": 0.6 },
            "directional": { "color": "#FFF5E6", "intensity": 1.2, "position": [150, 200, 100] },
            "fog": { "color": params.palette.sky_gradient[1], "near": 100.0, "far": 1200.0 }
        },
        "skybox_def": {
            "gradient": params.palette.sky_gradient,
            "sun_position": [100.0, 50.0, -200.0]
        }
    }))
}

fn op_assemble_modular_rig(params: AssembleModularRigParams) -> Result<Value, String> {
    let mut interactive_parts = Vec::new();
    let mut sockets_map = serde_json::Map::new();

    for s in &params.sockets {
        sockets_map.insert(s.socket_id.clone(), json!(s.transform));
        if s.socket_id.contains("door") || s.socket_id.contains("wheel") {
            interactive_parts.push(s.socket_id.clone());
        }
    }

    let camera_anchors = json!({
        "cockpit": [0.0, params.base_chassis.dimensions[1] * 0.4, params.base_chassis.dimensions[2] * 0.35],
        "chase": [0.0, params.base_chassis.dimensions[1] * 1.5, -params.base_chassis.dimensions[2] * 2.2],
        "panoramic_side": [params.base_chassis.dimensions[0] * 1.8, 1.0, 0.0]
    });

    Ok(json!({
        "status": "success",
        "hierarchy_graph": {
            "root": "base_chassis",
            "dimensions": params.base_chassis.dimensions,
            "sockets": sockets_map,
            "attachments_count": params.attachments.len(),
            "occupant_capacity": params.interior_occupants.len()
        },
        "interactive_parts": interactive_parts,
        "camera_anchors": camera_anchors
    }))
}

fn op_simulate_path_kinematics(params: SimulatePathKinematicsParams) -> Result<Value, String> {
    let specs = params.vehicle_specs.unwrap_or(VehicleSpecs {
        max_speed: 25.0,
        power_accel: 3.5,
        brake_decel: 7.0,
        suspension_roll_factor: 0.6,
    });

    let dt = params.delta_time.clamp(0.001, 0.1);
    let throttle_accel = params.control_intent.throttle.clamp(0.0, 1.0) * specs.power_accel;
    let brake_decel = params.control_intent.brake.clamp(0.0, 1.0) * specs.brake_decel;

    // Gravity component along track grade
    let gravity = 9.81;
    let grade_force = -gravity * params.track_metrics.grade_angle.sin();

    // Aerodynamic / Rolling drag
    let drag = 0.03 * params.transform_state.velocity.powi(2);

    let net_accel = throttle_accel - brake_decel + grade_force - drag;
    let next_vel = (params.transform_state.velocity + net_accel * dt).clamp(0.0, specs.max_speed);

    // Lateral acceleration: a_lat = v^2 * curvature
    let lateral_accel = next_vel * next_vel * params.track_metrics.curvature;
    let lateral_g = lateral_accel / gravity;

    // Body roll estimation (Euler roll in radians)
    let body_roll_rad = -lateral_g * specs.suspension_roll_factor;

    let delta_progress = (next_vel * dt) / 1000.0; // scaled normalized advancement
    let next_t = (params.transform_state.t_normalized + delta_progress) % 1.0;

    Ok(json!({
        "status": "success",
        "next_state": {
            "t_normalized": next_t,
            "velocity": next_vel,
            "acceleration": net_accel
        },
        "body_roll_euler": [0.0, 0.0, body_roll_rad],
        "g_force_vector": [lateral_g, 1.0 + (net_accel / gravity), 0.0],
        "lateral_accel": lateral_accel
    }))
}

fn op_evaluate_dynamic_comfort(params: EvaluateDynamicComfortParams) -> Result<Value, String> {
    let thresholds = params.thresholds.unwrap_or(ComfortThresholds {
        max_comfortable_g: 0.25,
        jerk_penalty_rate: 5.0,
    });

    let mut penalty = 0.0;
    let mut penalty_applied = false;

    for sample in &params.g_force_history {
        if sample.lateral_g > thresholds.max_comfortable_g {
            let excess = sample.lateral_g - thresholds.max_comfortable_g;
            penalty += excess * 15.0;
            penalty_applied = true;
        }
        if sample.jerk > 2.0 {
            penalty += (sample.jerk - 2.0) * thresholds.jerk_penalty_rate;
            penalty_applied = true;
        }
    }

    let updated_score = (params.current_comfort_score - penalty).clamp(0.0, 100.0);
    let mut updated_streak = params.streak_status.clone();

    let event_dispatched = if penalty_applied && updated_streak.active {
        updated_streak.active = false;
        updated_streak.count = 0;
        Some("Streak broken. Passenger discomfort detected due to abrupt lateral acceleration or braking.".to_string())
    } else if !penalty_applied && updated_score >= 80.0 {
        // Activate streak if not already active
        if !updated_streak.active {
            updated_streak.active = true;
        }
        updated_streak.count += 1;
        None
    } else {
        None
    };

    Ok(json!({
        "status": "success",
        "comfort_score": updated_score,
        "penalty_applied": penalty_applied,
        "streak_status": updated_streak,
        "event_dispatched": event_dispatched
    }))
}

fn op_manage_zone_lifecycle(params: ManageZoneLifecycleParams) -> Result<Value, String> {
    let mut notifications = Vec::new();
    let mut score_delta = 0.0;

    let phase = match (params.zone_type.as_str(), params.trigger_event.as_str()) {
        ("station", "enter") => {
            notifications.push(json!({
                "text": "Approaching station platform. Reduce velocity.",
                "duration": 3.0,
                "style": "info"
            }));
            "approaching"
        }
        ("station", "dock") => {
            let bonus = params
                .zone_context
                .as_ref()
                .and_then(|c| c.bonus_values)
                .unwrap_or(100.0);
            score_delta += bonus;
            notifications.push(json!({
                "text": "Docked. Doors opening. Boarding passengers.",
                "duration": 4.0,
                "style": "success"
            }));
            "docked"
        }
        ("station", "exit") => {
            notifications.push(json!({
                "text": "Departing station. Track clear ahead.",
                "duration": 2.5,
                "style": "info"
            }));
            "departed"
        }
        ("checkpoint", "enter") => {
            score_delta += 50.0;
            notifications.push(json!({
                "text": "Checkpoint reached.",
                "duration": 2.0,
                "style": "bonus"
            }));
            "checkpoint_cleared"
        }
        ("hazard", "enter") => {
            notifications.push(json!({
                "text": "Caution: Track section requires reduced speed.",
                "duration": 3.0,
                "style": "warning"
            }));
            "hazard_active"
        }
        _ => "nominal",
    };

    Ok(json!({
        "status": "success",
        "lifecycle_phase": phase,
        "entity_mutations": {
            "doors_open": phase == "docked",
            "can_accelerate": phase != "docked"
        },
        "ui_notifications": notifications,
        "score_delta": score_delta
    }))
}

fn op_synthesize_audio_graph(params: SynthesizeAudioGraphParams) -> Result<Value, String> {
    let (code, routing) = match params.audio_preset.as_str() {
        "engine_drone" => (
            r#"
const ctx = new (window.AudioContext || window.webkitAudioContext)();
const osc = ctx.createOscillator();
const gain = ctx.createGain();
osc.type = 'sawtooth';
osc.frequency.setValueAtTime(65.0, ctx.currentTime);
gain.gain.setValueAtTime(0.15, ctx.currentTime);
osc.connect(gain);
gain.connect(ctx.destination);
osc.start();
window.engineAudio = { ctx, osc, gain, setThrottle: (t) => {
    osc.frequency.setTargetAtTime(60.0 + (t * 120.0), ctx.currentTime, 0.05);
    gain.gain.setTargetAtTime(0.1 + (t * 0.1), ctx.currentTime, 0.05);
}};
"#
            .to_string(),
            json!({
                "oscillators": ["sawtooth (65Hz)"],
                "gain_nodes": ["master_engine_gain"],
                "control_hooks": ["setThrottle(0..1)"]
            }),
        ),
        "friction_surface" => (
            r#"
const ctx = new (window.AudioContext || window.webkitAudioContext)();
const bufferSize = ctx.sampleRate * 2;
const noiseBuffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
const output = noiseBuffer.getChannelData(0);
for (let i = 0; i < bufferSize; i++) { output[i] = Math.random() * 2 - 1; }
const whiteNoise = ctx.createBufferSource();
whiteNoise.buffer = noiseBuffer;
whiteNoise.loop = true;
const filter = ctx.createBiquadFilter();
filter.type = 'bandpass';
filter.frequency.setValueAtTime(1200, ctx.currentTime);
filter.Q.setValueAtTime(3.0, ctx.currentTime);
const gain = ctx.createGain();
gain.gain.setValueAtTime(0.0, ctx.currentTime);
whiteNoise.connect(filter);
filter.connect(gain);
gain.connect(ctx.destination);
whiteNoise.start();
window.frictionAudio = { ctx, gain, filter, setSpeed: (v) => {
    gain.gain.setTargetAtTime(Math.min(v / 30.0, 0.25), ctx.currentTime, 0.05);
    filter.frequency.setTargetAtTime(800 + (v * 40), ctx.currentTime, 0.05);
}};
"#
            .to_string(),
            json!({
                "noise_buffers": ["white_noise_2sec_loop"],
                "filters": ["bandpass_1200Hz_q3"],
                "control_hooks": ["setSpeed(v)"]
            }),
        ),
        "chime" => (
            r#"
window.playChime = () => {
    const ctx = new (window.AudioContext || window.webkitAudioContext)();
    [523.25, 659.25, 783.99].forEach((freq, idx) => {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = 'sine';
        osc.frequency.setValueAtTime(freq, ctx.currentTime + idx * 0.12);
        gain.gain.setValueAtTime(0.2, ctx.currentTime + idx * 0.12);
        gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + idx * 0.12 + 1.2);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start(ctx.currentTime + idx * 0.12);
        osc.stop(ctx.currentTime + idx * 0.12 + 1.25);
    });
};
"#
            .to_string(),
            json!({
                "oscillators": ["sine_c5", "sine_e5", "sine_g5"],
                "envelopes": ["exponential_decay_1.2s"],
                "control_hooks": ["playChime()"]
            }),
        ),
        _ => (
            r#"
const ctx = new (window.AudioContext || window.webkitAudioContext)();
const osc = ctx.createOscillator();
const gain = ctx.createGain();
osc.type = 'triangle';
osc.frequency.setValueAtTime(220, ctx.currentTime);
gain.gain.setValueAtTime(0.05, ctx.currentTime);
osc.connect(gain);
gain.connect(ctx.destination);
osc.start();
"#
            .to_string(),
            json!({ "oscillators": ["triangle_220Hz"] }),
        ),
    };

    Ok(json!({
        "status": "success",
        "audio_preset": params.audio_preset,
        "webaudio_nodes_code": code.trim(),
        "routing_graph": routing
    }))
}

fn op_bundle_webgl_container(params: BundleWebglContainerParams) -> Result<Value, String> {
    let scripts = params.engine_scripts.join("\n\n");
    let theme_css = params.ui_layout.theme_css.unwrap_or_else(|| {
        ":root { --bg: #141824; --text: #E8ECEF; --accent: #E06D53; font-family: sans-serif; }"
            .to_string()
    });

    let html_document = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>Rune WebGL Simulation</title>
  <style>
    {theme_css}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body, html {{ width: 100%; height: 100%; overflow: hidden; background: var(--bg); }}
    #viewport {{ width: 100%; height: 100%; display: block; }}
    #hud-overlay {{ position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; }}
    .interactive {{ pointer-events: auto; }}
  </style>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/three.js/r128/three.min.js"></script>
</head>
<body>
  <canvas id="viewport"></canvas>
  <div id="hud-overlay">
    <div id="status-card" class="interactive"></div>
  </div>
  <script>
    // --- Engine Runtime Scripts ---
    {scripts}
  </script>
</body>
</html>"#
    );

    let bundle_size = html_document.len();

    Ok(json!({
        "status": "success",
        "html_document": html_document,
        "bundle_size_bytes": bundle_size,
        "renderer": params.renderer,
        "asset_integrity": true
    }))
}

fn op_inspect_webgl_performance(params: InspectWebglPerformanceParams) -> Result<Value, String> {
    let html_len = params.html_bundle.len();
    let has_threejs =
        params.html_bundle.contains("three.min.js") || params.html_bundle.contains("THREE.");
    let script_tags = params.html_bundle.matches("<script").count();

    // Simple heuristic static analysis
    let estimated_draw_calls = if has_threejs {
        45 + (script_tags * 10) as u32
    } else {
        12
    };
    let estimated_memory_mb = (html_len as f64 / (1024.0 * 1024.0)) + 38.0;

    let mut warnings = Vec::new();
    if estimated_draw_calls > params.draw_call_limit {
        warnings.push(format!(
            "Estimated draw calls ({}) exceed limit ({})",
            estimated_draw_calls, params.draw_call_limit
        ));
    }
    if !has_threejs && params.html_bundle.contains("WebGLRenderer") {
        warnings.push("WebGLRenderer referenced without bundled runtime library".to_string());
    }

    let pass_status = warnings.is_empty();

    Ok(json!({
        "status": "success",
        "pass_status": pass_status,
        "metrics": {
            "estimated_draw_calls": estimated_draw_calls,
            "estimated_memory_mb": (estimated_memory_mb * 100.0).round() / 100.0,
            "bundle_size_bytes": html_len,
            "target_fps": params.target_fps
        },
        "warnings": warnings
    }))
}
