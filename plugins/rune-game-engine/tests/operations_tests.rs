use rune_game_engine::operations::execute_tool;
use rune_pdk::ToolCallRequest;
use serde_json::json;

#[test]
fn test_generate_path_network_catmull_rom() {
    let req = ToolCallRequest {
        name: "generate_path_network".to_string(),
        arguments: json!({
            "nodes": [
                { "id": "n1", "position": [0.0, 0.0, 0.0] },
                { "id": "n2", "position": [10.0, 5.0, 20.0] },
                { "id": "n3", "position": [20.0, 0.0, 50.0] }
            ],
            "curve_type": "catmull_rom",
            "closed": false,
            "cross_section": "rail"
        }),
    };

    let res = execute_tool(req).expect("Failed to execute generate_path_network");
    assert_eq!(res["status"], "success");
    assert!(res["path_data"]["total_length"].as_f64().unwrap() > 0.0);
    assert!(!res["path_data"]["waypoints"].as_array().unwrap().is_empty());
}

#[test]
fn test_generate_path_network_closed() {
    let req = ToolCallRequest {
        name: "generate_path_network".to_string(),
        arguments: json!({
            "nodes": [
                { "id": "n1", "position": [0.0, 0.0, 0.0] },
                { "id": "n2", "position": [10.0, 5.0, 10.0] }
            ],
            "curve_type": "bezier",
            "closed": true,
            "cross_section": "road"
        }),
    };

    let res = execute_tool(req).expect("Failed to execute generate_path_network");
    assert_eq!(res["status"], "success");
    assert_eq!(res["path_data"]["closed"], true);
}

#[test]
fn test_generate_path_network_insufficient_nodes() {
    let req = ToolCallRequest {
        name: "generate_path_network".to_string(),
        arguments: json!({
            "nodes": [
                { "id": "n1", "position": [0.0, 0.0, 0.0] }
            ],
            "curve_type": "catmull_rom"
        }),
    };

    let res = execute_tool(req).expect_err("Should fail with insufficient nodes");
    assert!(res.contains("at least 2 nodes"));
}

#[test]
fn test_generate_procedural_biome_floating_islands() {
    let req = ToolCallRequest {
        name: "generate_procedural_biome".to_string(),
        arguments: json!({
            "terrain_type": "floating_islands",
            "palette": {
                "sky_gradient": ["#87CEEB", "#2C3E50"],
                "ambient": "#FFF5E6",
                "surface_colors": ["#8B4513", "#228B22", "#CD853F"]
            },
            "feature_scatter": [
                { "asset_type": "tree", "density": 0.5 },
                { "asset_type": "house", "density": 0.2 }
            ],
            "boundary_box": [[0, -20, 0], [100, 20, 100]]
        }),
    };

    let res = execute_tool(req).expect("Failed to execute generate_procedural_biome");
    assert_eq!(res["status"], "success");
    assert_eq!(res["scene_manifest"]["biome_type"], "floating_islands");
    assert!(
        !res["scene_manifest"]["features"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_assemble_modular_rig_tram() {
    let req = ToolCallRequest {
        name: "assemble_modular_rig".to_string(),
        arguments: json!({
            "base_chassis": {
                "dimensions": [4.0, 2.5, 2.0],
                "pivot_offset": [0.0, 0.0, 0.0]
            },
            "sockets": [
                { "socket_id": "wheel_fl", "transform": [1.0, 0.0, -1.0] },
                { "socket_id": "wheel_fr", "transform": [-1.0, 0.0, -1.0] }
            ],
            "attachments": [
                { "socket_id": "roof", "component_def": {}, "position": [0.0, 2.0, 0.0] }
            ],
            "interior_occupants": []
        }),
    };

    let res = execute_tool(req).expect("Failed to execute assemble_modular_rig");
    assert_eq!(res["status"], "success");
    assert_eq!(res["assembly_info"]["socket_count"], 2);
    assert_eq!(res["assembly_info"]["attachment_count"], 1);
}

#[test]
fn test_simulate_path_kinematics_and_roll() {
    let req = ToolCallRequest {
        name: "simulate_path_kinematics".to_string(),
        arguments: json!({
            "transform_state": {
                "t_normalized": 0.0,
                "velocity": 5.0,
                "acceleration": 0.0
            },
            "control_intent": {
                "throttle": 0.8,
                "brake": 0.0
            },
            "track_metrics": {
                "curvature": 0.1,
                "grade_angle": 0.0
            },
            "vehicle_specs": {
                "max_speed": 25.0,
                "power_accel": 3.5,
                "brake_decel": 7.0,
                "suspension_roll_factor": 0.6
            }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute simulate_path_kinematics");
    assert_eq!(res["status"], "success");
    assert!(
        !res["simulation"]["trajectory"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!res["simulation"]["events"].as_array().unwrap().is_empty());
}

#[test]
fn test_simulate_path_kinematics_with_grade() {
    let req = ToolCallRequest {
        name: "simulate_path_kinematics".to_string(),
        arguments: json!({
            "transform_state": {
                "t_normalized": 0.5,
                "velocity": 10.0,
                "acceleration": 0.0
            },
            "control_intent": {
                "throttle": 0.5,
                "brake": 0.0
            },
            "track_metrics": {
                "curvature": 0.05,
                "grade_angle": 0.1
            }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute simulate_path_kinematics");
    assert_eq!(res["status"], "success");
}

#[test]
fn test_simulate_path_kinematics_with_braking() {
    let req = ToolCallRequest {
        name: "simulate_path_kinematics".to_string(),
        arguments: json!({
            "transform_state": {
                "t_normalized": 0.8,
                "velocity": 20.0,
                "acceleration": 0.0
            },
            "control_intent": {
                "throttle": 0.0,
                "brake": 0.8
            }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute simulate_path_kinematics");
    assert_eq!(res["status"], "success");
}

#[test]
fn test_evaluate_dynamic_comfort_good_ride() {
    let req = ToolCallRequest {
        name: "evaluate_dynamic_comfort".to_string(),
        arguments: json!({
            "g_force_history": [
                { "lateral_g": 0.2, "vertical_g": 1.0, "jerk": 0.1 },
                { "lateral_g": 0.3, "vertical_g": 1.0, "jerk": 0.15 }
            ],
            "current_comfort_score": 90.0,
            "streak_status": { "active": false, "count": 0 }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute evaluate_dynamic_comfort");
    assert_eq!(res["status"], "success");
    assert!(
        res["comfort_report"]["rating"].as_str().unwrap() == "excellent"
            || res["comfort_report"]["rating"].as_str().unwrap() == "good"
    );
}

#[test]
fn test_evaluate_dynamic_comfort_penalty() {
    let req = ToolCallRequest {
        name: "evaluate_dynamic_comfort".to_string(),
        arguments: json!({
            "g_force_history": [
                { "lateral_g": 1.5, "vertical_g": 1.0, "jerk": 2.0 },
                { "lateral_g": 2.0, "vertical_g": 1.0, "jerk": 3.0 }
            ],
            "current_comfort_score": 50.0,
            "streak_status": { "active": true, "count": 2 }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute evaluate_dynamic_comfort");
    assert_eq!(res["status"], "success");
    assert!(
        !res["comfort_report"]["violations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_manage_zone_lifecycle_station() {
    let req = ToolCallRequest {
        name: "manage_zone_lifecycle".to_string(),
        arguments: json!({
            "zone_type": "station",
            "trigger_event": "enter",
            "entity_state": { "state": "docked" }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute manage_zone_lifecycle");
    assert_eq!(res["status"], "success");
    assert_eq!(
        res["zone_state"]["zones"].as_array().unwrap()[0]["type"],
        "station"
    );
    assert_eq!(
        res["zone_state"]["zones"].as_array().unwrap()[0]["state"],
        "inactive"
    );
}

#[test]
fn test_manage_zone_lifecycle_checkpoint() {
    let req = ToolCallRequest {
        name: "manage_zone_lifecycle".to_string(),
        arguments: json!({
            "zone_type": "checkpoint",
            "trigger_event": "pass",
            "entity_state": { "state": "cleared" }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute manage_zone_lifecycle");
    assert_eq!(res["status"], "success");
    assert_eq!(
        res["zone_state"]["zones"].as_array().unwrap()[0]["type"],
        "checkpoint"
    );
}

#[test]
fn test_manage_zone_lifecycle_hazard() {
    let req = ToolCallRequest {
        name: "manage_zone_lifecycle".to_string(),
        arguments: json!({
            "zone_type": "hazard",
            "trigger_event": "exit",
            "entity_state": { "state": "clear" }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute manage_zone_lifecycle");
    assert_eq!(res["status"], "success");
    assert_eq!(
        res["zone_state"]["zones"].as_array().unwrap()[0]["type"],
        "hazard"
    );
}

#[test]
fn test_synthesize_audio_graph_engine() {
    let req = ToolCallRequest {
        name: "synthesize_audio_graph".to_string(),
        arguments: json!({
            "audio_preset": "engine_drone",
            "driver_parameters": {}
        }),
    };

    let res = execute_tool(req).expect("Failed to execute synthesize_audio_graph");
    assert_eq!(res["status"], "success");
    assert_eq!(res["scenario"], "engine_drone");
    assert!(res["audio_code"].as_str().unwrap().contains("AudioContext"));
}

#[test]
fn test_synthesize_audio_graph_wind() {
    let req = ToolCallRequest {
        name: "synthesize_audio_graph".to_string(),
        arguments: json!({
            "audio_preset": "environmental_wind",
            "driver_parameters": {}
        }),
    };

    let res = execute_tool(req).expect("Failed to execute synthesize_audio_graph");
    assert_eq!(res["status"], "success");
    assert_eq!(res["scenario"], "environmental_wind");
}

#[test]
fn test_bundle_webgl_canvas2d() {
    let req = ToolCallRequest {
        name: "bundle_webgl_container".to_string(),
        arguments: json!({
            "renderer": "threejs",
            "engine_scripts": [],
            "ui_layout": {
                "hud_anchors": {},
                "theme_css": null,
                "control_mode": "touch_and_keys"
            },
            "embedded_assets": {}
        }),
    };

    let res = execute_tool(req).expect("Failed to execute bundle_webgl_container");
    assert_eq!(res["status"], "success");
    assert!(
        res["html_bundle"]
            .as_str()
            .unwrap()
            .contains("WebGL Container")
    );
}

#[test]
fn test_bundle_and_inspect_webgl_container() {
    let bundle_req = ToolCallRequest {
        name: "bundle_webgl_container".to_string(),
        arguments: json!({
            "renderer": "threejs",
            "engine_scripts": [],
            "ui_layout": {
                "hud_anchors": {},
                "theme_css": "body { background: black; }",
                "control_mode": "touch_and_keys"
            },
            "embedded_assets": {}
        }),
    };

    let res = execute_tool(bundle_req).expect("Failed to execute bundle_webgl_container");
    assert_eq!(res["status"], "success");
    assert_eq!(
        res["bundle_size_bytes"],
        res["html_bundle"].as_str().unwrap().len()
    );

    let inspect_req = ToolCallRequest {
        name: "inspect_webgl_performance".to_string(),
        arguments: json!({
            "html_bundle": res["html_bundle"].as_str().unwrap().to_string(),
            "target_fps": 60,
            "draw_call_limit": 150,
            "simulated_devices": []
        }),
    };

    let inspect_res =
        execute_tool(inspect_req).expect("Failed to execute inspect_webgl_performance");
    assert_eq!(inspect_res["status"], "success");
}
