use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdExecRequest {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdExecResponse {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareImagesRequest {
    pub image1_path: String,
    pub image2_path: String,
    #[serde(default)]
    pub output_path: String,
    #[serde(default)]
    pub algorithm: String,
    #[serde(default)]
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonStats {
    pub width: u32,
    pub height: u32,
    pub pixels_compared: usize,
    pub matching_pixels: usize,
    pub differing_pixels: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareImagesResponse {
    pub match_percentage: f64,
    pub differences_found: usize,
    pub output_path: String,
    pub algorithm_used: String,
    pub comparison_stats: ComparisonStats,
}
