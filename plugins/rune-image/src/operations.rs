use crate::types::{CmdExecRequest, CmdExecResponse, CompareImagesResponse, ComparisonStats};
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

fn rasterize_svg(_svg_path: &str, _width: u32, _height: u32) -> Result<DynamicImage, String> {
    // Try to load SVG directly with image crate
    // Note: image crate doesn't have native SVG support in all cases
    // This is a fallback that works for simple SVGs

    match image::open(_svg_path) {
        Ok(img) => {
            // Image was loaded successfully
            Ok(img)
        }
        Err(_e) => {
            // If image crate can't load SVG, provide clear error message
            Err("SVG file could not be loaded. The image crate doesn't support SVG natively. For full SVG support, please install rsvg-convert system tool or use a pure Rust SVG library.".to_string())
        }
    }
}

fn is_svg_file(path: &str) -> bool {
    path.to_lowercase().ends_with(".svg")
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

            // Compare images
            let (diff_image, stats) = generate_diff_image(&img1, &img2, threshold);

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

        unknown => Err(format!("Unknown tool: {}", unknown)),
    }
}
