use serde::{Deserialize, Serialize};

// --- generate_path_network types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathNode {
    pub id: String,
    pub position: [f64; 3],
    #[serde(default)]
    pub banking: f64,
    #[serde(default)]
    pub tangent: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileOptions {
    #[serde(default = "default_gauge")]
    pub gauge_width: f64,
    #[serde(default = "default_spacing")]
    pub tie_spacing: f64,
    #[serde(default = "default_interval")]
    pub pylon_interval: f64,
}

fn default_gauge() -> f64 {
    2.0
}
fn default_spacing() -> f64 {
    1.5
}
fn default_interval() -> f64 {
    30.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratePathNetworkParams {
    pub nodes: Vec<PathNode>,
    #[serde(default = "default_curve_type")]
    pub curve_type: String, // "catmull_rom" | "bezier" | "bspline"
    #[serde(default)]
    pub closed: bool,
    #[serde(default = "default_cross_section")]
    pub cross_section: String, // "rail" | "pipe" | "road" | "custom"
    #[serde(default)]
    pub profile_options: Option<ProfileOptions>,
}

fn default_curve_type() -> String {
    "catmull_rom".to_string()
}
fn default_cross_section() -> String {
    "rail".to_string()
}

// --- generate_procedural_biome types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureScatter {
    pub asset_type: String,
    pub density: f64,
    #[serde(default)]
    pub cluster_rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomePalette {
    pub sky_gradient: [String; 2],
    pub ambient: String,
    pub surface_colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateProceduralBiomeParams {
    pub terrain_type: String, // "floating_islands" | "heightmap" | "archipelago"
    pub palette: BiomePalette,
    #[serde(default)]
    pub feature_scatter: Vec<FeatureScatter>,
    pub boundary_box: [[f64; 3]; 2],
}

// --- assemble_modular_rig types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChassisDef {
    pub dimensions: [f64; 3],
    #[serde(default)]
    pub pivot_offset: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketDef {
    pub socket_id: String,
    pub transform: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentDef {
    #[serde(alias = "socket_id", alias = "id")]
    pub socket_id: String,
    #[serde(default)]
    pub component_def: serde_json::Value,
    #[serde(default)]
    pub position: Option<[f64; 3]>,
    #[serde(default, alias = "type")]
    pub attachment_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleModularRigParams {
    pub base_chassis: ChassisDef,
    #[serde(default)]
    pub sockets: Vec<SocketDef>,
    #[serde(default)]
    pub attachments: Vec<AttachmentDef>,
    #[serde(default)]
    pub interior_occupants: Vec<serde_json::Value>,
}

// --- simulate_path_kinematics types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformState {
    pub t_normalized: f64,
    pub velocity: f64,
    #[serde(default)]
    pub acceleration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlIntent {
    #[serde(default)]
    pub throttle: f64, // 0.0 to 1.0
    #[serde(default)]
    pub brake: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackMetrics {
    #[serde(default)]
    pub curvature: f64,
    #[serde(default)]
    pub grade_angle: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalForces {
    #[serde(default)]
    pub vector: [f64; 3],
    #[serde(default)]
    pub drag_coeff: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleSpecs {
    #[serde(default = "default_max_speed")]
    pub max_speed: f64,
    #[serde(default = "default_power_accel")]
    pub power_accel: f64,
    #[serde(default = "default_brake_decel")]
    pub brake_decel: f64,
    #[serde(default = "default_suspension_roll")]
    pub suspension_roll_factor: f64,
}

fn default_max_speed() -> f64 {
    25.0
}
fn default_power_accel() -> f64 {
    3.5
}
fn default_brake_decel() -> f64 {
    7.0
}
fn default_suspension_roll() -> f64 {
    0.6
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatePathKinematicsParams {
    pub transform_state: TransformState,
    pub control_intent: ControlIntent,
    #[serde(default)]
    pub track_metrics: TrackMetrics,
    #[serde(default)]
    pub external_forces: Option<ExternalForces>,
    #[serde(default)]
    pub vehicle_specs: Option<VehicleSpecs>,
    #[serde(default = "default_dt")]
    pub delta_time: f64,
}

fn default_dt() -> f64 {
    0.0166
}

// --- evaluate_dynamic_comfort types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GForceSample {
    pub lateral_g: f64,
    pub vertical_g: f64,
    pub jerk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComfortThresholds {
    #[serde(default = "default_max_comfortable_g")]
    pub max_comfortable_g: f64,
    #[serde(default = "default_jerk_penalty_rate")]
    pub jerk_penalty_rate: f64,
}

fn default_max_comfortable_g() -> f64 {
    0.25
}
fn default_jerk_penalty_rate() -> f64 {
    5.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakStatus {
    pub active: bool,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateDynamicComfortParams {
    pub g_force_history: Vec<GForceSample>,
    pub current_comfort_score: f64,
    #[serde(default)]
    pub thresholds: Option<ComfortThresholds>,
    pub streak_status: StreakStatus,
}

// --- manage_zone_lifecycle types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneContext {
    #[serde(default)]
    pub queue_count: u32,
    #[serde(default)]
    pub subtitles: Option<String>,
    #[serde(default)]
    pub bonus_values: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManageZoneLifecycleParams {
    pub zone_type: String,     // "station" | "checkpoint" | "hazard" | "workshop"
    pub trigger_event: String, // "enter" | "dock" | "exit"
    pub entity_state: serde_json::Value,
    #[serde(default)]
    pub zone_context: Option<ZoneContext>,
}

// --- synthesize_audio_graph types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeAudioGraphParams {
    pub audio_preset: String, // "engine_drone" | "friction_surface" | "chime" | "environmental_wind"
    #[serde(default)]
    pub driver_parameters: serde_json::Map<String, serde_json::Value>,
}

// --- bundle_webgl_container types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiLayout {
    pub hud_anchors: serde_json::Value,
    #[serde(default)]
    pub theme_css: Option<String>,
    #[serde(default = "default_control_mode")]
    pub control_mode: String,
}

fn default_control_mode() -> String {
    "touch_and_keys".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleWebglContainerParams {
    #[serde(default = "default_renderer")]
    pub renderer: String,
    pub engine_scripts: Vec<String>,
    pub ui_layout: UiLayout,
    #[serde(default)]
    pub embedded_assets: Option<serde_json::Map<String, serde_json::Value>>,
}

fn default_renderer() -> String {
    "threejs".to_string()
}

// --- inspect_webgl_performance types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectWebglPerformanceParams {
    pub html_bundle: String,
    #[serde(default = "default_target_fps")]
    pub target_fps: u32,
    #[serde(default = "default_draw_call_limit")]
    pub draw_call_limit: u32,
    #[serde(default)]
    pub simulated_devices: Vec<String>,
}

fn default_target_fps() -> u32 {
    60
}
fn default_draw_call_limit() -> u32 {
    150
}

// --- Optimized Declarative Orchestration Workflow Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    #[serde(alias = "type")]
    pub r#type: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileDeclarativeBundleParams {
    pub engine: String,
    pub theme: String,
    pub pipeline: Vec<PipelineStep>,
    #[serde(default)]
    pub assets: Option<serde_json::Value>,
    #[serde(default)]
    pub ui_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileDeclarativeBundleResult {
    pub status: String,
    pub html_document: String,
    pub bundle_size_bytes: u32,
    pub validation_report: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPipelineExecutorParams {
    pub steps: Vec<PipelineStep>,
    pub input_state: serde_json::Value,
    #[serde(default)]
    pub parallel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPipelineExecutorResult {
    pub status: String,
    pub outputs: serde_json::Value,
    pub execution_order: Vec<String>,
    pub total_time_ms: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchAstNodeParams {
    pub file_path: String,
    pub node_type: String,
    pub node_name: String,
    pub operation: String,
    pub new_value: serde_json::Value,
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchAstNodeResult {
    pub status: String,
    pub file_path: String,
    pub changes_made: Vec<String>,
    pub new_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateHeadlessRuntimeParams {
    pub html_bundle: String,
    pub test_cases: Vec<String>,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(default)]
    pub headless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateHeadlessRuntimeResult {
    pub status: String,
    pub passed: bool,
    pub console_errors: Vec<String>,
    pub console_warnings: Vec<String>,
    pub performance_metrics: serde_json::Value,
    pub screenshot_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTemplateRegistryParams {
    pub template_name: String,
    pub template_type: String,
    #[serde(default)]
    pub custom_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTemplateRegistryResult {
    pub status: String,
    pub template: String,
    pub cache_hit: bool,
    pub template_size_bytes: u32,
}

// --- Gameplay & Experience Enhancement Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputScriptEvent {
    #[serde(alias = "time")]
    pub time_ms: u64,
    #[serde(default)]
    pub keys_down: Vec<String>,
    #[serde(default)]
    pub keys_up: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateVirtualPlaytestParams {
    pub html_bundle: String,
    #[serde(default)]
    pub input_script: Vec<InputScriptEvent>,
    #[serde(default)]
    pub metrics_to_track: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureCameraControllerParams {
    pub camera_mode: String, // "follow_rail" | "spring_arm" | "cockpit" | "isometric"
    #[serde(default)]
    pub target_entity_id: String,
    pub damping: serde_json::Value,
    pub dynamic_fov: serde_json::Value,
    pub look_ahead: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackChannel {
    pub name: String,
    #[serde(default)]
    pub intensity_scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMapping {
    pub trigger: String,
    pub intensity_metric: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeGameFeelSystemParams {
    pub feedback_channels: Vec<FeedbackChannel>,
    pub event_mappings: Vec<EventMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunableParameter {
    pub key: String,
    pub path: String,
    pub r#type: String, // "number" | "boolean" | "select"
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuneGameplayParametersParams {
    pub harness_type: String, // "tweakpane" | "dat_gui" | "headless_config"
    pub tunable_registry: Vec<TunableParameter>,
    #[serde(default)]
    pub preset_profiles: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputVariable {
    pub name: String,
    pub domain: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceMechanicEconomyParams {
    pub input_variables: Vec<InputVariable>,
    pub target_metric: String, // "comfort_score" | "tip_reward" | "streak_decay"
    pub curve_type: String,    // "exponential" | "logarithmic" | "sigmoid" | "piecewise"
    #[serde(default)]
    pub forgiveness_window_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDef {
    pub material_id: String,
    pub r#type: String,
    #[serde(default)]
    pub shaders: Option<serde_json::Value>,
    #[serde(default)]
    pub uniforms: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditShaderAndMaterialPipelineParams {
    pub scene_materials: Vec<MaterialDef>,
    pub environment_features: serde_json::Value,
}
