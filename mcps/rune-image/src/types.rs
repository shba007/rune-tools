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
    #[serde(rename = "image1Path", alias = "image1_path")]
    pub image1_path: String,
    #[serde(rename = "image2Path", alias = "image2_path")]
    pub image2_path: String,
    #[serde(rename = "outputPath", alias = "output_path", default)]
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
    #[serde(rename = "pixelsCompared", alias = "pixels_compared")]
    pub pixels_compared: usize,
    #[serde(rename = "matchingPixels", alias = "matching_pixels")]
    pub matching_pixels: usize,
    #[serde(rename = "differingPixels", alias = "differing_pixels")]
    pub differing_pixels: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareImagesResponse {
    #[serde(rename = "matchPercentage", alias = "match_percentage")]
    pub match_percentage: f64,
    #[serde(rename = "differencesFound", alias = "differences_found")]
    pub differences_found: usize,
    #[serde(rename = "outputPath", alias = "output_path")]
    pub output_path: String,
    #[serde(rename = "algorithmUsed", alias = "algorithm_used")]
    pub algorithm_used: String,
    #[serde(rename = "comparisonStats", alias = "comparison_stats")]
    pub comparison_stats: ComparisonStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadataResponse {
    pub format: String,
    pub width: u32,
    pub height: u32,
    #[serde(rename = "colorType", alias = "color_type")]
    pub color_type: String,
    #[serde(rename = "hasAlpha", alias = "has_alpha")]
    pub has_alpha: bool,
    #[serde(rename = "bitDepth", alias = "bit_depth")]
    pub bit_depth: u8,
    #[serde(rename = "fileSize", alias = "file_size")]
    pub file_size: u64,
    pub dimensions: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertImageResponse {
    pub success: bool,
    #[serde(rename = "outputPath", alias = "output_path")]
    pub output_path: String,
    #[serde(rename = "inputFormat", alias = "input_format")]
    pub input_format: String,
    #[serde(rename = "outputFormat", alias = "output_format")]
    pub output_format: String,
    pub message: String,
}
