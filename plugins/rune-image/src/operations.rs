use crate::types::{
    CmdExecRequest, CmdExecResponse, CompareImagesResponse, ComparisonStats, ConvertImageResponse,
    ImageMetadataResponse,
};
use image::GenericImage;
use image::GenericImageView;
use image::{DynamicImage, Rgba};
use rune_pdk::ToolCallRequest;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

// SVG rasterization constants
const DEFAULT_SVG_WIDTH: u32 = 800;
const DEFAULT_SVG_HEIGHT: u32 = 800;

#[cfg(target_arch = "wasm32")]
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_cmd_exec(input: String) -> String;
}

fn get_str_arg(args: &Value, camel: &str, snake: &str) -> Option<String> {
    if let Some(val) = args
        .get(camel)
        .or_else(|| args.get(snake))
        .and_then(Value::as_str)
    {
        return Some(val.to_string());
    }
    let env_snake = snake.to_ascii_uppercase();
    let env_camel = camel.to_ascii_uppercase();
    std::env::var(&env_snake)
        .or_else(|_| std::env::var(&env_camel))
        .or_else(|_| {
            if snake == "output_directory" {
                std::env::var("OUTPUT_DIR").or_else(|_| std::env::var("ALLOWED_DIR"))
            } else {
                Err(std::env::VarError::NotPresent)
            }
        })
        .ok()
}

pub fn get_config(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        extism_pdk::config::get(key)
            .ok()
            .flatten()
            .or_else(|| std::env::var(key.to_ascii_uppercase()).ok())
            .filter(|s| !s.is_empty())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(key.to_ascii_uppercase())
            .or_else(|_| std::env::var(key))
            .ok()
            .filter(|s| !s.is_empty())
    }
}

#[cfg(target_arch = "wasm32")]
fn run_binary_raw(req: &CmdExecRequest) -> Result<CmdExecResponse, String> {
    let raw_req = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let raw_resp =
        unsafe { host_cmd_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;
    serde_json::from_str(&raw_resp).map_err(|e| format!("Failed to parse host response: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_binary_raw(req: &CmdExecRequest) -> Result<CmdExecResponse, String> {
    let mut cmd = std::process::Command::new(&req.program);
    cmd.args(&req.args);
    if let Some(ref cwd) = req.cwd {
        cmd.current_dir(cwd);
    }
    match cmd.output() {
        Ok(output) => Ok(CmdExecResponse {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(e) => Ok(CmdExecResponse {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("Failed to spawn {}: {}", req.program, e),
        }),
    }
}

pub fn run_binary(program: &str, args: &[&str], cwd: Option<&str>) -> Result<String, String> {
    let req = CmdExecRequest {
        program: program.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd: cwd.map(|c| c.to_string()),
    };

    let resp = run_binary_raw(&req)?;

    if resp.success {
        Ok(resp.stdout.trim().to_string())
    } else {
        let err = if !resp.stderr.trim().is_empty() {
            resp.stderr.trim()
        } else if !resp.stdout.trim().is_empty() {
            resp.stdout.trim()
        } else {
            "Process exited with non-zero exit code"
        };
        Err(format!("Error executing '{}': {}", program, err))
    }
}

pub fn resolve_dir(dir_param: Option<&str>) -> String {
    let explicit = dir_param.map(ToString::to_string).or_else(|| {
        std::env::var("OUTPUT_DIRECTORY")
            .or_else(|_| std::env::var("OUTPUT_DIR"))
            .or_else(|_| std::env::var("ALLOWED_DIR"))
            .ok()
    });

    let raw = explicit.unwrap_or_else(|| ".".to_string());
    let target = PathBuf::from(raw);

    if let Some(allowed_root) = get_config("allowed_dir") {
        let root = PathBuf::from(allowed_root);
        if target.is_relative() {
            root.join(target).to_string_lossy().to_string()
        } else {
            target.to_string_lossy().to_string()
        }
    } else {
        target.to_string_lossy().to_string()
    }
}

pub fn extract_domain_and_stem(url: &str) -> (String, String) {
    let without_proto = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))
        .unwrap_or(url.trim());

    let host = without_proto
        .split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .to_lowercase();

    let clean_domain = host.trim_start_matches("www.").to_string();
    let stem = clean_domain
        .split('.')
        .next()
        .unwrap_or(&clean_domain)
        .to_string();

    (clean_domain, stem)
}

pub fn find_cookie_file_in_dir(dir_path: &Path, url: &str) -> Option<PathBuf> {
    if !dir_path.exists() || !dir_path.is_dir() {
        return None;
    }

    let (domain, stem) = extract_domain_and_stem(url);
    let candidates = [
        format!("{}.txt", domain),
        format!("{}.txt", stem),
        format!("{}_cookies.txt", domain),
        format!("{}_cookies.txt", stem),
        format!("{}-cookies.txt", stem),
        format!("cookies-{}.txt", stem),
    ];

    for candidate in &candidates {
        let path = dir_path.join(candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type()
                && ft.is_file()
            {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.ends_with(".txt") && (name.contains(&stem) || name.contains(&domain)) {
                    return Some(entry.path());
                }
            }
        }
    }

    let default_cookie = dir_path.join("cookies.txt");
    if default_cookie.is_file() {
        return Some(default_cookie);
    }

    None
}

pub fn resolve_cookie_arg(params: &ToolCallRequest, url: &str) -> Option<(String, String)> {
    let explicit_file = get_str_arg(&params.arguments, "cookiesFile", "cookies_file");
    if let Some(file) = explicit_file {
        return Some(("--cookies".to_string(), file));
    }

    let cookies_dir = get_str_arg(&params.arguments, "cookiesDir", "cookies_dir");
    if let Some(dir_str) = cookies_dir {
        let dir_path = PathBuf::from(resolve_dir(Some(&dir_str)));
        if let Some(matched_file) = find_cookie_file_in_dir(&dir_path, url) {
            return Some((
                "--cookies".to_string(),
                matched_file.to_string_lossy().to_string(),
            ));
        }
    }

    let browser = get_str_arg(
        &params.arguments,
        "cookiesFromBrowser",
        "cookies_from_browser",
    );
    if let Some(b) = browser {
        return Some(("--cookies-from-browser".to_string(), b));
    }

    None
}

fn apply_gallerydl_access_args<'a>(
    args: &mut Vec<&'a str>,
    params: &'a ToolCallRequest,
    url: &str,
    storage: &'a mut Vec<String>,
) {
    if let Some((flag, val)) = resolve_cookie_arg(params, url) {
        storage.push(flag);
        storage.push(val);
    }

    if let Some(p) = get_str_arg(&params.arguments, "proxy", "proxy") {
        storage.push("--proxy".to_string());
        storage.push(p);
    }

    for s in storage.iter() {
        args.push(s.as_str());
    }
}

fn parse_gallerydl_output(raw_output: &str) -> Vec<String> {
    let mut urls = Vec::new();

    if let Ok(root) = serde_json::from_str::<Value>(raw_output) {
        collect_urls_from_value(&root, &mut urls);
    } else {
        let stream = serde_json::Deserializer::from_str(raw_output).into_iter::<Value>();
        for val in stream.flatten() {
            collect_urls_from_value(&val, &mut urls);
        }
    }

    let mut deduped = Vec::new();
    for u in urls {
        if !deduped.contains(&u) {
            deduped.push(u);
        }
    }
    deduped
}

fn collect_urls_from_value(val: &Value, urls: &mut Vec<String>) {
    if let Some(arr) = val.as_array() {
        if !arr.is_empty() && arr[0].is_number() {
            if let Some(s) = arr.get(1).and_then(Value::as_str)
                && (s.starts_with("http://") || s.starts_with("https://"))
            {
                urls.push(s.to_string());
            }
        } else {
            for elem in arr {
                collect_urls_from_value(elem, urls);
            }
        }
    } else if let Some(obj) = val.as_object() {
        for key in ["url", "file_url", "image", "preview_url", "src"] {
            if let Some(s) = obj.get(key).and_then(Value::as_str)
                && (s.starts_with("http://") || s.starts_with("https://"))
            {
                urls.push(s.to_string());
                break;
            }
        }
    }
}

fn compare_images_pixel_by_pixel(
    img1: &DynamicImage,
    img2: &DynamicImage,
    threshold: f64,
) -> (DynamicImage, ComparisonStats) {
    let (width1, height1) = img1.dimensions();
    let (width2, height2) = img2.dimensions();

    let target_width = width1.max(width2);
    let target_height = height1.max(height2);

    let img1_resized = image::imageops::resize(
        img1,
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );
    let img2_resized = image::imageops::resize(
        img2,
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    let mut diff_img = DynamicImage::new_rgb8(target_width, target_height);
    let mut matching_pixels = 0usize;
    let mut differing_pixels = 0usize;

    for y in 0..target_height {
        for x in 0..target_width {
            let pixel1 = img1_resized.get_pixel(x, y);
            let pixel2 = img2_resized.get_pixel(x, y);

            let r1 = pixel1[0] as i32;
            let g1 = pixel1[1] as i32;
            let b1 = pixel1[2] as i32;

            let r2 = pixel2[0] as i32;
            let g2 = pixel2[1] as i32;
            let b2 = pixel2[2] as i32;

            let dr = (r1 - r2).abs();
            let dg = (g1 - g2).abs();
            let db = (b1 - b2).abs();

            // Calculate Euclidean distance
            let distance = ((dr * dr + dg * dg + db * db) as f64).sqrt();
            let max_diff = distance / 255.0;

            if max_diff < threshold {
                matching_pixels += 1;
                diff_img.put_pixel(x, y, Rgba([50, 200, 50, 255])); // Green for match
            } else {
                differing_pixels += 1;
                let intensity = (max_diff * 255.0) as u8;
                diff_img.put_pixel(
                    x,
                    y,
                    Rgba([intensity, 255 - intensity, 255 - intensity, 255]),
                ); // Red/blue for difference
            }
        }
    }

    let stats = ComparisonStats {
        width: target_width,
        height: target_height,
        pixels_compared: matching_pixels + differing_pixels,
        matching_pixels,
        differing_pixels,
    };

    (diff_img, stats)
}

fn rasterize_svg(svg_path: &str, width: u32, height: u32) -> Result<DynamicImage, String> {
    // The `image` crate has no SVG decoder, so we rasterize with `resvg`
    // (pure Rust, no system libraries needed -- works in wasm32 and native).
    if width == 0 || height == 0 {
        return Err(format!(
            "Failed to convert SVG '{}': target dimensions must be non-zero, got {}x{}",
            svg_path, width, height
        ));
    }

    let svg_data =
        fs::read(svg_path).map_err(|e| format!("Failed to convert SVG '{}': {}", svg_path, e))?;

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&svg_data, &options)
        .map_err(|e| format!("Failed to convert SVG '{}': {}", svg_path, e))?;

    let svg_size = tree.size();
    let (svg_width, svg_height) = (svg_size.width(), svg_size.height());
    if svg_width <= 0.0 || svg_height <= 0.0 {
        return Err(format!(
            "Failed to convert SVG '{}': document has zero intrinsic size",
            svg_path
        ));
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        format!(
            "Failed to convert SVG '{}': could not allocate a {}x{} canvas",
            svg_path, width, height
        )
    })?;

    // Scale the SVG's own coordinate space to fill the requested raster size.
    let transform = resvg::tiny_skia::Transform::from_scale(
        width as f32 / svg_width,
        height as f32 / svg_height,
    );

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny-skia stores premultiplied-alpha pixels; `image` expects straight
    // alpha, so we demultiply each pixel on the way out.
    let raw_pixels: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();

    let buf = image::RgbaImage::from_raw(width, height, raw_pixels).ok_or_else(|| {
        format!(
            "Failed to convert SVG '{}': rasterized buffer size mismatch",
            svg_path
        )
    })?;

    Ok(DynamicImage::ImageRgba8(buf))
}

fn is_svg_file(path: &str) -> bool {
    path.to_lowercase().ends_with(".svg")
}

/// Builds a vtracer `Config` from optional tool arguments, layered on top of
/// vtracer's own defaults (color mode, spline curves, stacked hierarchy --
/// already tuned for photos/color art, not just line drawings).
fn build_trace_config(args: &Value) -> vtracer::Config {
    let mut config = vtracer::Config::default();

    if let Some(v) = get_str_arg(args, "traceColorMode", "trace_color_mode") {
        config.color_mode = match v.to_lowercase().as_str() {
            "binary" | "bw" => vtracer::ColorMode::Binary,
            _ => vtracer::ColorMode::Color,
        };
    }

    if let Some(v) = get_str_arg(args, "traceHierarchical", "trace_hierarchical") {
        config.hierarchical = match v.to_lowercase().as_str() {
            "cutout" => vtracer::Hierarchical::Cutout,
            _ => vtracer::Hierarchical::Stacked,
        };
    }

    if let Some(v) = get_str_arg(args, "traceCurveMode", "trace_curve_mode") {
        config.mode = match v.to_lowercase().as_str() {
            "polygon" => visioncortex::PathSimplifyMode::Polygon,
            "none" | "pixel" => visioncortex::PathSimplifyMode::None,
            _ => visioncortex::PathSimplifyMode::Spline,
        };
    }

    if let Some(v) =
        get_str_arg(args, "colorPrecision", "color_precision").and_then(|s| s.parse::<i32>().ok())
    {
        config.color_precision = v;
    }
    if let Some(v) =
        get_str_arg(args, "filterSpeckle", "filter_speckle").and_then(|s| s.parse::<usize>().ok())
    {
        config.filter_speckle = v;
    }
    if let Some(v) =
        get_str_arg(args, "layerDifference", "layer_difference").and_then(|s| s.parse::<i32>().ok())
    {
        config.layer_difference = v;
    }
    if let Some(v) =
        get_str_arg(args, "cornerThreshold", "corner_threshold").and_then(|s| s.parse::<i32>().ok())
    {
        config.corner_threshold = v;
    }
    if let Some(v) =
        get_str_arg(args, "lengthThreshold", "length_threshold").and_then(|s| s.parse::<f64>().ok())
    {
        config.length_threshold = v;
    }
    if let Some(v) =
        get_str_arg(args, "spliceThreshold", "splice_threshold").and_then(|s| s.parse::<i32>().ok())
    {
        config.splice_threshold = v;
    }
    if let Some(v) =
        get_str_arg(args, "maxIterations", "max_iterations").and_then(|s| s.parse::<usize>().ok())
    {
        config.max_iterations = v;
    }
    if let Some(v) =
        get_str_arg(args, "pathPrecision", "path_precision").and_then(|s| s.parse::<u32>().ok())
    {
        config.path_precision = Some(v);
    }

    config
}

fn generate_diff_image(
    img1: &DynamicImage,
    img2: &DynamicImage,
    threshold: f64,
) -> (DynamicImage, ComparisonStats) {
    compare_images_pixel_by_pixel(img1, img2, threshold)
}

fn load_and_convert_image(
    path: &str,
    default_width: u32,
    default_height: u32,
) -> Result<(DynamicImage, u32, u32), String> {
    if is_svg_file(path) {
        let (width, height) = if path.contains("width=") && path.contains("height=") {
            // Parse dimensions from path if available
            let width_part = path.split("width=").nth(1).unwrap_or("");
            let height_part = path.split("height=").nth(1).unwrap_or("");

            let width_str = width_part
                .split(" ")
                .next()
                .unwrap_or(&default_width.to_string())
                .to_string();
            let width_val: u32 = width_str.parse().unwrap_or(default_width);

            let height_str = height_part
                .split(" ")
                .next()
                .unwrap_or(&default_height.to_string())
                .to_string();
            let height_val: u32 = height_str.parse().unwrap_or(default_height);

            (width_val, height_val)
        } else {
            (default_width, default_height)
        };
        let img = rasterize_svg(path, width, height)?;
        Ok((img, width, height))
    } else {
        let img = image::open(path).map_err(|e| format!("Failed to open image: {}", e))?;
        let (width, height) = img.dimensions();
        Ok((img, width, height))
    }
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let mut str_storage: Vec<String> = Vec::new();

    match request.name.as_str() {
        "inspect_image_gallery" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let mut args = vec!["--dump-json"];
            apply_gallerydl_access_args(&mut args, &request, &url, &mut str_storage);
            args.push(&url);

            let out = run_binary("gallery-dl", &args, None)?;
            let previews = parse_gallerydl_output(&out);

            let count = previews.len();
            Ok(json!({
                "gallery_url": url,
                "total_media_found": count,
                "previews": previews.into_iter().take(5).collect::<Vec<_>>()
            }))
        }

        "download_image_collection" => {
            let url = get_str_arg(&request.arguments, "url", "url")
                .ok_or_else(|| "Missing 'url' parameter".to_string())?;

            if url.trim().is_empty() {
                return Err("Parameter 'url' cannot be empty".to_string());
            }

            let output_dir_param =
                get_str_arg(&request.arguments, "outputDirectory", "output_directory");
            let output_dir = resolve_dir(output_dir_param.as_deref());

            let _ = fs::create_dir_all(&output_dir);

            let range = get_str_arg(&request.arguments, "filterRange", "filter_range");

            let mut args = vec!["--directory", &output_dir];
            apply_gallerydl_access_args(&mut args, &request, &url, &mut str_storage);

            if let Some(ref r) = range {
                args.push("--range");
                args.push(r);
            }
            args.push(&url);

            let out = run_binary("gallery-dl", &args, None)?;
            Ok(json!({ "status": "success", "output": out }))
        }

        "compare_images" => {
            let image1_path = get_str_arg(&request.arguments, "image1_path", "image1Path")
                .ok_or_else(|| "Missing 'image1_path' parameter".to_string())?;

            let image2_path = get_str_arg(&request.arguments, "image2_path", "image2Path")
                .ok_or_else(|| "Missing 'image2_path' parameter".to_string())?;

            let output_path = get_str_arg(&request.arguments, "output_path", "outputPath")
                .unwrap_or_else(|| "./diff.png".to_string());

            let algorithm = get_str_arg(&request.arguments, "algorithm", "Algorithm")
                .unwrap_or_else(|| "rms".to_string());

            let threshold_str = get_str_arg(&request.arguments, "threshold", "Threshold")
                .unwrap_or_else(|| "0.0".to_string());

            let threshold: f64 = threshold_str
                .parse()
                .map_err(|_| "Invalid threshold value. Must be a number between 0.0 and 1.0")?;

            // Use minimum threshold of 0.001 to account for floating point precision in pixel comparisons
            let effective_threshold = threshold.max(0.001);

            // Validate threshold before loading images
            if effective_threshold > 1.0 {
                return Err("Threshold must be between 0.0 and 1.0".to_string());
            }

            if image1_path.trim().is_empty() || image2_path.trim().is_empty() {
                return Err("Image paths cannot be empty".to_string());
            }

            // Check if files exist
            if !Path::new(&image1_path).exists() {
                return Err(format!("Image1 path does not exist: {}", image1_path));
            }
            if !Path::new(&image2_path).exists() {
                return Err(format!("Image2 path does not exist: {}", image2_path));
            }

            // Load and convert images
            let (img1, _width1, _height1) =
                load_and_convert_image(&image1_path, DEFAULT_SVG_WIDTH, DEFAULT_SVG_HEIGHT)?;
            let (img2, _width2, _height2) =
                load_and_convert_image(&image2_path, DEFAULT_SVG_WIDTH, DEFAULT_SVG_HEIGHT)?;

            // Compare images. Note: we intentionally pass `effective_threshold`
            // here, not the raw `threshold`. With the raw threshold (which
            // defaults to 0.0) and a strict `<` comparison, even pixels with
            // zero difference would fail `0.0 < 0.0` and be counted as
            // "differing" -- which made comparing a file against itself
            // report a 0% match. `effective_threshold` floors this at 0.001
            // specifically to absorb that floating-point edge case.
            let (diff_image, stats) = generate_diff_image(&img1, &img2, effective_threshold);

            // Save diff image
            let output_path_resolved = PathBuf::from(&output_path).to_string_lossy().to_string();

            let _ = fs::create_dir_all(
                PathBuf::from(&output_path_resolved)
                    .parent()
                    .unwrap_or(Path::new(".")),
            );
            diff_image
                .save(&output_path_resolved)
                .map_err(|e| format!("Failed to save diff image: {}", e))?;

            // Calculate match percentage
            let match_percentage =
                (stats.matching_pixels as f64 / stats.pixels_compared as f64) * 100.0;

            Ok(json!(CompareImagesResponse {
                match_percentage,
                differences_found: stats.differing_pixels,
                output_path: output_path_resolved,
                algorithm_used: algorithm,
                comparison_stats: stats
            }))
        }

        "get_image_metadata" => {
            let image_path = get_str_arg(&request.arguments, "image_path", "imagePath")
                .ok_or_else(|| "Missing 'image_path' parameter".to_string())?;

            if image_path.trim().is_empty() {
                return Err("Parameter 'image_path' cannot be empty".to_string());
            }

            if !Path::new(&image_path).exists() {
                return Err(format!("Image path does not exist: {}", image_path));
            }

            let img =
                image::open(&image_path).map_err(|e| format!("Failed to open image: {}", e))?;

            let (width, height) = img.dimensions();

            // Get format from image extension or default
            let format_str = Path::new(&image_path)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_uppercase())
                .unwrap_or_else(|| "Unknown".to_string());

            let color_type = match img {
                DynamicImage::ImageRgba8(_) => "RGBA".to_string(),
                DynamicImage::ImageRgb8(_) => "RGB".to_string(),
                DynamicImage::ImageLuma8(_) => "Luma".to_string(),
                DynamicImage::ImageLumaA8(_) => "Luma with Alpha".to_string(),
                DynamicImage::ImageRgb16(_) => "RGB16".to_string(),
                DynamicImage::ImageRgba16(_) => "RGBA16".to_string(),
                _ => "Unknown".to_string(),
            };
            let has_alpha = matches!(
                img,
                DynamicImage::ImageRgba8(_)
                    | DynamicImage::ImageLumaA8(_)
                    | DynamicImage::ImageRgba16(_)
            );
            let file_metadata = std::fs::metadata(&image_path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?;
            let file_size = file_metadata.len();

            Ok(json!(ImageMetadataResponse {
                format: format_str,
                width,
                height,
                color_type,
                has_alpha,
                bit_depth: 8, // Simplified - could be enhanced
                file_size,
                dimensions: format!("{}x{}", width, height),
                metadata: json!({})
            }))
        }

        "convert_image_format" => {
            let input_path = get_str_arg(&request.arguments, "input_path", "inputPath")
                .ok_or_else(|| "Missing 'input_path' parameter".to_string())?;

            let output_format = get_str_arg(&request.arguments, "output_format", "outputFormat")
                .ok_or_else(|| "Missing 'output_format' parameter".to_string())?
                .to_lowercase();

            let _quality = get_str_arg(&request.arguments, "quality", "quality")
                .and_then(|q| q.parse().ok())
                .unwrap_or(0.9);

            if input_path.trim().is_empty() || output_format.trim().is_empty() {
                return Err("Input path and output format cannot be empty".to_string());
            }

            if !Path::new(&input_path).exists() {
                return Err(format!("Input path does not exist: {}", input_path));
            }

            let input_ext = Path::new(&input_path)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_else(|| "unknown".to_string());

            let output_path = if let Some(parent) = Path::new(&input_path).parent() {
                parent.join(format!(
                    "{}_converted.{}",
                    Path::new(&input_path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or("output"),
                    output_format
                ))
            } else {
                PathBuf::from(format!(
                    "{}_converted.{}",
                    Path::new(&input_path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or("output"),
                    output_format
                ))
            };

            // SVG to raster conversion
            if input_ext == "svg" {
                if !["png", "jpeg", "jpg", "gif", "webp"].contains(&output_format.as_str()) {
                    return Err(format!(
                        "SVG can only be converted to raster formats: png, jpeg, gif, webp. Got: {}",
                        output_format
                    ));
                }

                // Use rasterize_svg function (already implemented)
                let rasterized = rasterize_svg(&input_path, 800, 800)?;

                let ext = match output_format.as_str() {
                    "jpeg" | "jpg" => "jpeg",
                    _ => &output_format,
                };

                rasterized
                    .save(output_path.with_extension(ext))
                    .map_err(|e| format!("Failed to save converted image: {}", e))?;
            } else if output_format == "svg" {
                // Raster to SVG vectorization via vtracer. Unlike potrace,
                // vtracer has a color clustering pipeline, so it's suited to
                // photos and colored art, not just black & white line work.
                let config = build_trace_config(&request.arguments);
                vtracer::convert_image_to_svg(Path::new(&input_path), &output_path, config)
                    .map_err(|e| format!("Failed to vectorize image to SVG: {}", e))?;
            } else {
                // Raster to raster conversion. Only open with `image::open`
                // here (not for SVG above): the `image` crate has no SVG
                // decoder, so calling it unconditionally on an SVG file
                // used to fail before this branch was even reached.
                let img = image::open(&input_path)
                    .map_err(|e| format!("Failed to open input image: {}", e))?;

                // The image crate handles format detection based on file extension
                img.save(&output_path)
                    .map_err(|e| format!("Failed to save converted image: {}", e))?;
            }

            let message = if output_format == "svg" {
                "Image vectorized to SVG successfully".to_string()
            } else {
                "Image converted successfully".to_string()
            };

            Ok(json!(ConvertImageResponse {
                success: true,
                output_path: output_path.to_string_lossy().to_string(),
                input_format: input_ext,
                output_format,
                message
            }))
        }

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
