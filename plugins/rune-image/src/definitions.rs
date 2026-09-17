use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "inspect_image_gallery".to_string(),
            description: "Scans albums, artist profiles, or social posts (Reddit, Instagram, Imgur, Pixiv) with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Gallery or post URL"
                    },
                    "cookiesDir": {
                        "type": "string",
                        "description": "Directory containing site cookie files (e.g. reddit.txt, cookies.txt)"
                    },
                    "cookiesFromBrowser": {
                        "type": "string",
                        "description": "Browser to load session cookies from"
                    },
                    "cookiesFile": {
                        "type": "string",
                        "description": "Explicit path to a cookies.txt file"
                    },
                    "proxy": {
                        "type": "string",
                        "description": "HTTP/HTTPS/SOCKS proxy URL"
                    }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "download_image_collection".to_string(),
            description: "Downloads image galleries, multi-image posts, or artist boards with cookie dir auto-matching.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Gallery URL"
                    },
                    "filterRange": {
                        "type": "string",
                        "description": "Range of items to download (e.g. '1-10')"
                    },
                    "outputDirectory": {
                        "type": "string",
                        "description": "Target directory for saved images"
                    },
                    "cookiesDir": {
                        "type": "string",
                        "description": "Directory containing site cookie files (e.g. reddit.txt, cookies.txt)"
                    },
                    "cookiesFromBrowser": {
                        "type": "string",
                        "description": "Browser to load session cookies from"
                    },
                    "cookiesFile": {
                        "type": "string",
                        "description": "Explicit path to a cookies.txt file"
                    },
                    "proxy": {
                        "type": "string",
                        "description": "HTTP/HTTPS/SOCKS proxy URL"
                    }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "compare_images".to_string(),
            description: "Compares two images pixel-by-pixel and generates a visual difference image showing mismatches. Supports multiple algorithms (RMS for exact match, MSSIM for structural similarity, perceptual for human-perceived differences).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "image1_path": {
                        "type": "string",
                        "description": "Path to the first image file (PNG, JPEG, GIF, SVG or WebP)"
                    },
                    "image2_path": {
                        "type": "string",
                        "description": "Path to the second image file (PNG, JPEG, GIF, SVG or WebP)"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Path for the difference image output. Defaults to './diff.png'."
                    },
                    "algorithm": {
                        "type": "string",
                        "description": "Comparison algorithm: 'rms' (pixel-by-pixel exact match), 'mssim' (structural similarity), 'perceptual' (human-perceived differences). Defaults to 'rms'."
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Threshold for considering pixels different (0.0-1.0). Lower = more strict. Defaults to 0.0 (exact match)."
                    }
                },
                "required": ["image1_path", "image2_path"]
            }),
        },
        ToolDefinition {
            name: "get_image_metadata".to_string(),
            description: "Extracts detailed metadata from an image file including format, dimensions, color space, compression, and other technical details.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "image_path": {
                        "type": "string",
                        "description": "Path to the image file"
                    }
                },
                "required": ["image_path"]
            }),
        },
        ToolDefinition {
            name: "convert_image_format".to_string(),
            description: "Converts an image from one format to another. Supports conversion between raster formats (PNG, JPEG, GIF, WebP), SVG-to-raster rasterization, and raster-to-SVG vectorization (via vtracer, suited to photos and color art -- not just black & white line drawings).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input_path": {
                        "type": "string",
                        "description": "Path to the input image file"
                    },
                    "output_format": {
                        "type": "string",
                        "description": "Target format: 'png', 'jpeg', 'gif', 'webp', or 'svg'. SVG input can be converted to any raster format; raster input can be vectorized to 'svg'."
                    },
                    "quality": {
                        "type": "number",
                        "description": "Output quality (0.0-1.0) for lossy formats like JPEG. Defaults to 0.9.",
                        "minimum": 0.0,
                        "maximum": 1.0
                    },
                    "trace_color_mode": {
                        "type": "string",
                        "description": "Raster-to-SVG only. 'color' (default, keeps full color -- best for photos) or 'binary' (single color, faster, best for line art)."
                    },
                    "trace_hierarchical": {
                        "type": "string",
                        "description": "Raster-to-SVG only. 'stacked' (default) layers shapes; 'cutout' avoids overlapping shapes. Only applies in color mode."
                    },
                    "trace_curve_mode": {
                        "type": "string",
                        "description": "Raster-to-SVG only. Curve fitting: 'spline' (default, smooth curves), 'polygon' (straight segments), or 'none' (pixel-aligned, good for pixel art)."
                    },
                    "color_precision": {
                        "type": "integer",
                        "description": "Raster-to-SVG only. Significant bits per RGB channel; higher preserves more color detail. Defaults to 6."
                    },
                    "filter_speckle": {
                        "type": "integer",
                        "description": "Raster-to-SVG only. Discards traced patches smaller than this many pixels, to suppress noise. Defaults to 4."
                    },
                    "layer_difference": {
                        "type": "integer",
                        "description": "Raster-to-SVG only. Color difference threshold between gradient layers. Defaults to 16."
                    },
                    "corner_threshold": {
                        "type": "integer",
                        "description": "Raster-to-SVG only. Minimum angle (degrees) to treat a point as a corner rather than smoothing it. Defaults to 60."
                    },
                    "length_threshold": {
                        "type": "number",
                        "description": "Raster-to-SVG only. Subdivides curves until segments are shorter than this length. Range [3.5, 10]. Defaults to 4.0."
                    },
                    "splice_threshold": {
                        "type": "integer",
                        "description": "Raster-to-SVG only. Minimum angle displacement (degrees) to splice a spline. Defaults to 45."
                    },
                    "max_iterations": {
                        "type": "integer",
                        "description": "Raster-to-SVG only. Maximum smoothing iterations. Defaults to 10."
                    },
                    "path_precision": {
                        "type": "integer",
                        "description": "Raster-to-SVG only. Decimal places used in the output path coordinates. Defaults to 2."
                    }
                },
                "required": ["input_path", "output_format"]
            }),
        },
    ]
}
