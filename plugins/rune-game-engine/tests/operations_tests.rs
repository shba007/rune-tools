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
    assert!(res["total_distance"].as_f64().unwrap() > 0.0);
    assert!(res["sample_count"].as_u64().unwrap() >= 20);
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
    // Check that closed path is generated
    assert!(res["path_points"].as_array().is_some());
    assert!(res["frames"].as_array().is_some());
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
    assert!(res.contains("requires at least 2 control nodes"));
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
    assert_eq!(res["terrain_type"], "floating_islands");
    assert!(!res["scene_manifest"].as_array().unwrap().is_empty());
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
                { "socket_id": "door_front", "transform": [0.5, 0.0, 0.0] },
                { "socket_id": "luggage_rack", "transform": [0.0, 1.5, 0.0] }
            ],
            "attachments": [
                { "socket_id": "luggage_rack", "component_def": { "type": "wood", "style": "vintage" } },
                { "id": "roof_luggage", "type": "luggage_rack", "position": [0.0, 1.9, 0.0] },
                { "id": "lantern_front", "type": "lantern", "position": [4.9, 2.3, 0.0] }
            ],
            "interior_occupants": [
                { "seat_id": "seat_1", "type": "passenger" },
                { "seat_id": "seat_2", "type": "passenger" }
            ]
        }),
    };

    let res = execute_tool(req).expect("Failed to execute assemble_modular_rig");
    assert_eq!(res["status"], "success");
    assert!(
        res["interactive_parts"]
            .as_array()
            .unwrap()
            .contains(&json!("door_front"))
    );
    assert_eq!(res["hierarchy_graph"]["occupant_capacity"], 2);
}

#[test]
fn test_simulate_path_kinematics_and_roll() {
    let req = ToolCallRequest {
        name: "simulate_path_kinematics".to_string(),
        arguments: json!({
            "transform_state": {
                "t_normalized": 0.25,
                "velocity": 12.0,
                "acceleration": 0.0
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
        }),
    };

    let res = execute_tool(req).expect("Failed to execute simulate_path_kinematics");
    assert_eq!(res["status"], "success");
    // Velocity should be close to initial value (small acceleration over 16ms)
    let new_vel = res["next_state"]["velocity"].as_f64().unwrap();
    assert!((new_vel - 12.0).abs() < 0.5);
    // Body roll should be calculated
    assert!(res["body_roll_euler"][2].is_f64());
}

#[test]
fn test_simulate_path_kinematics_with_braking() {
    let req = ToolCallRequest {
        name: "simulate_path_kinematics".to_string(),
        arguments: json!({
            "transform_state": {
                "t_normalized": 0.5,
                "velocity": 20.0
            },
            "control_intent": {
                "throttle": 0.0,
                "brake": 0.8
            },
            "track_metrics": {
                "curvature": 0.0,
                "grade_angle": 0.0
            },
            "delta_time": 0.0166
        }),
    };

    let res = execute_tool(req).expect("Failed to execute simulate_path_kinematics");
    assert_eq!(res["status"], "success");
    assert!(res["next_state"]["velocity"].as_f64().unwrap() < 20.0);
}

#[test]
fn test_simulate_path_kinematics_with_grade() {
    let req = ToolCallRequest {
        name: "simulate_path_kinematics".to_string(),
        arguments: json!({
            "transform_state": {
                "t_normalized": 0.3,
                "velocity": 10.0
            },
            "control_intent": {
                "throttle": 0.0,
                "brake": 0.0
            },
            "track_metrics": {
                "curvature": 0.0,
                "grade_angle": 0.1
            },
            "delta_time": 0.0166
        }),
    };

    let res = execute_tool(req).expect("Failed to execute simulate_path_kinematics");
    assert_eq!(res["status"], "success");
    // Going uphill should reduce velocity due to gravity
    assert!(res["next_state"]["velocity"].as_f64().unwrap() < 10.0);
}

#[test]
fn test_evaluate_dynamic_comfort_penalty() {
    let req = ToolCallRequest {
        name: "evaluate_dynamic_comfort".to_string(),
        arguments: json!({
            "g_force_history": [
                { "lateral_g": 0.55, "vertical_g": 1.0, "jerk": 3.2 }
            ],
            "current_comfort_score": 90.0,
            "streak_status": {
                "active": true,
                "count": 10
            }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute evaluate_dynamic_comfort");
    assert_eq!(res["status"], "success");
    assert_eq!(res["penalty_applied"], true);
    assert_eq!(res["streak_status"]["active"], false);
    assert!(
        res["event_dispatched"]
            .as_str()
            .unwrap()
            .contains("Streak broken")
    );
}

#[test]
fn test_evaluate_dynamic_comfort_good_ride() {
    let req = ToolCallRequest {
        name: "evaluate_dynamic_comfort".to_string(),
        arguments: json!({
            "g_force_history": [
                { "lateral_g": 0.1, "vertical_g": 1.0, "jerk": 0.5 }
            ],
            "current_comfort_score": 95.0,
            "streak_status": {
                "active": false,
                "count": 0
            }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute evaluate_dynamic_comfort");
    assert_eq!(res["status"], "success");
    // Should not apply penalty for gentle ride
    assert_eq!(res["penalty_applied"], false);
    // Streak should be active and increment
    assert_eq!(res["streak_status"]["active"], true);
    assert_eq!(res["streak_status"]["count"], 1);
}

#[test]
fn test_manage_zone_lifecycle_station() {
    let req = ToolCallRequest {
        name: "manage_zone_lifecycle".to_string(),
        arguments: json!({
            "zone_type": "station",
            "trigger_event": "dock",
            "entity_state": {},
            "zone_context": { "bonus_values": 100.0 }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute manage_zone_lifecycle");
    assert_eq!(res["status"], "success");
    assert_eq!(res["lifecycle_phase"], "docked");
    assert_eq!(res["score_delta"], 100.0);
    assert_eq!(res["entity_mutations"]["doors_open"], true);
}

#[test]
fn test_manage_zone_lifecycle_checkpoint() {
    let req = ToolCallRequest {
        name: "manage_zone_lifecycle".to_string(),
        arguments: json!({
            "zone_type": "checkpoint",
            "trigger_event": "enter",
            "entity_state": {}
        }),
    };

    let res = execute_tool(req).expect("Failed to execute manage_zone_lifecycle");
    assert_eq!(res["status"], "success");
    assert_eq!(res["lifecycle_phase"], "checkpoint_cleared");
    assert_eq!(res["score_delta"], 50.0);
}

#[test]
fn test_manage_zone_lifecycle_hazard() {
    let req = ToolCallRequest {
        name: "manage_zone_lifecycle".to_string(),
        arguments: json!({
            "zone_type": "hazard",
            "trigger_event": "enter",
            "entity_state": {}
        }),
    };

    let res = execute_tool(req).expect("Failed to execute manage_zone_lifecycle");
    assert_eq!(res["status"], "success");
    assert_eq!(res["lifecycle_phase"], "hazard_active");
    assert!(!res["ui_notifications"].as_array().unwrap().is_empty());
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
    assert!(
        res["webaudio_nodes_code"]
            .as_str()
            .unwrap()
            .contains("AudioContext")
    );
    assert!(
        res["webaudio_nodes_code"]
            .as_str()
            .unwrap()
            .contains("sawtooth")
    );
}

#[test]
fn test_synthesize_audio_graph_wind() {
    let req = ToolCallRequest {
        name: "synthesize_audio_graph".to_string(),
        arguments: json!({
            "audio_preset": "environmental_wind",
            "driver_parameters": { "speed_norm": 0.5 }
        }),
    };

    let res = execute_tool(req).expect("Failed to execute synthesize_audio_graph");
    assert_eq!(res["status"], "success");
    assert_eq!(res["audio_preset"], "environmental_wind");
}

#[test]
fn test_bundle_and_inspect_webgl_container() {
    let bundle_req = ToolCallRequest {
        name: "bundle_webgl_container".to_string(),
        arguments: json!({
            "renderer": "threejs",
            "engine_scripts": ["console.log('Engine initialized');"],
            "ui_layout": {
                "hud_anchors": { "top_left": "score" }
            }
        }),
    };

    let bundle_res = execute_tool(bundle_req).expect("Failed to bundle container");
    let html = bundle_res["html_document"].as_str().unwrap();
    assert!(html.contains("Engine initialized"));
    assert!(html.contains("three.min.js"));

    let inspect_req = ToolCallRequest {
        name: "inspect_webgl_performance".to_string(),
        arguments: json!({
            "html_bundle": html,
            "target_fps": 60,
            "draw_call_limit": 150
        }),
    };

    let inspect_res = execute_tool(inspect_req).expect("Failed to inspect performance");
    assert_eq!(inspect_res["pass_status"], true);
}

#[test]
fn test_bundle_webgl_canvas2d() {
    let req = ToolCallRequest {
        name: "bundle_webgl_container".to_string(),
        arguments: json!({
            "renderer": "canvas2d",
            "engine_scripts": ["const canvas = document.getElementById('viewport');"],
            "ui_layout": {
                "hud_anchors": { "bottom_center": "hud" }
            }
        }),
    };

    let res = execute_tool(req).expect("Failed to bundle canvas2d container");
    assert_eq!(res["status"], "success");
    assert_eq!(res["renderer"], "canvas2d");
}
