use crate::types::ScannerCapabilitiesResult;
use reqwest::blocking::Client;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    pub version: String,
    pub make_and_model: Option<String>,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
    pub supported_sources: Vec<String>,
    pub supported_intents: Vec<String>,
    pub supported_resolutions: Vec<u32>,
    pub supported_color_modes: Vec<String>,
    pub supported_formats: Vec<String>,
    pub platen_max_width: u32,
    pub platen_max_height: u32,
    pub adf_max_width: Option<u32>,
    pub adf_max_height: Option<u32>,
    pub raw_xml: String,
}

pub struct EsclClient {
    pub base_url: String,
    pub host_ip: String,
    client: Client,
}

impl EsclClient {
    pub fn new(scanner_ip: &str, port: u16, use_https: bool) -> Self {
        let scheme = if use_https { "https" } else { "http" };
        let base_url = if (use_https && port == 443) || (!use_https && port == 80) {
            format!("{}://{}", scheme, scanner_ip)
        } else {
            format!("{}://{}:{}", scheme, scanner_ip, port)
        };

        // Enforce HTTP/1.1 to match curl behavior and avoid HP embedded HTTP/2 POST failures
        let client = Client::builder()
            .http1_only()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        Self {
            base_url,
            host_ip: scanner_ip.to_string(),
            client,
        }
    }

    pub fn get_capabilities_raw(&self) -> Result<String, String> {
        let url = format!("{}/eSCL/ScannerCapabilities", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "curl/8.4.0")
            .header("Accept", "*/*")
            .send()
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("HTTP Status {}", resp.status()));
        }

        resp.text().map_err(|e| e.to_string())
    }

    pub fn parse_capabilities(&self) -> Result<DeviceCapabilities, String> {
        let xml = self.get_capabilities_raw()?;

        let version = find_tag_content(&xml, "pwg:Version")
            .unwrap_or_else(|| {
                find_tag_content(&xml, "Version")
                    .filter(|v| v.starts_with("2."))
                    .unwrap_or("2.63")
            })
            .to_string();

        let make_and_model = find_tag_content(&xml, "pwg:MakeAndModel")
            .or_else(|| find_tag_content(&xml, "MakeAndModel"))
            .map(|s| s.to_string());

        let manufacturer = find_tag_content(&xml, "scan:Manufacturer")
            .or_else(|| find_tag_content(&xml, "Manufacturer"))
            .map(|s| s.to_string());

        let serial_number = find_tag_content(&xml, "pwg:SerialNumber")
            .or_else(|| find_tag_content(&xml, "SerialNumber"))
            .map(|s| s.to_string());

        let mut supported_sources = Vec::new();
        if xml.contains("Platen") {
            supported_sources.push("Platen".to_string());
        }
        if xml.contains("Adf") {
            supported_sources.push("Adf".to_string());
        }

        let platen_xml = find_tag_content(&xml, "scan:Platen").unwrap_or(&xml);
        let platen_max_width = find_tag_content(platen_xml, "MaxWidth")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2550);
        let platen_max_height = find_tag_content(platen_xml, "MaxHeight")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3508);

        let (adf_max_width, adf_max_height) =
            if let Some(adf_xml) = find_tag_content(&xml, "scan:Adf") {
                (
                    find_tag_content(adf_xml, "MaxWidth").and_then(|v| v.parse::<u32>().ok()),
                    find_tag_content(adf_xml, "MaxHeight").and_then(|v| v.parse::<u32>().ok()),
                )
            } else {
                (None, None)
            };

        let mut supported_resolutions: Vec<u32> = find_all_tag_contents(&xml, "XResolution")
            .into_iter()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect();
        supported_resolutions.sort_unstable();
        supported_resolutions.dedup();
        if supported_resolutions.is_empty() {
            supported_resolutions = vec![75, 100, 150, 200, 300, 400, 600, 1200];
        }

        let mut supported_color_modes = find_all_tag_contents(&xml, "ColorMode");
        supported_color_modes.dedup();
        if supported_color_modes.is_empty() {
            supported_color_modes = vec![
                "RGB24".to_string(),
                "Grayscale8".to_string(),
                "BlackAndWhite1".to_string(),
            ];
        }

        let mut supported_formats = find_all_tag_contents(&xml, "DocumentFormat");
        supported_formats.dedup();
        if supported_formats.is_empty() {
            supported_formats = vec!["image/jpeg".to_string(), "application/pdf".to_string()];
        }

        let mut supported_intents = find_all_tag_contents(&xml, "Intent");
        supported_intents.dedup();
        if supported_intents.is_empty() {
            supported_intents = vec!["Document".to_string(), "Photo".to_string()];
        }

        Ok(DeviceCapabilities {
            version,
            make_and_model,
            manufacturer,
            serial_number,
            supported_sources,
            supported_intents,
            supported_resolutions,
            supported_color_modes,
            supported_formats,
            platen_max_width,
            platen_max_height,
            adf_max_width,
            adf_max_height,
            raw_xml: xml,
        })
    }

    pub fn query_capabilities(
        &self,
        scanner_ip: &str,
    ) -> Result<ScannerCapabilitiesResult, String> {
        let caps = self.parse_capabilities()?;

        Ok(ScannerCapabilitiesResult {
            scanner_ip: scanner_ip.to_string(),
            version: caps.version,
            make_and_model: caps.make_and_model,
            manufacturer: caps.manufacturer,
            serial_number: caps.serial_number,
            supported_sources: caps.supported_sources,
            supported_resolutions: caps.supported_resolutions,
            supported_color_modes: caps.supported_color_modes,
            supported_formats: caps.supported_formats,
            supported_intents: caps.supported_intents,
            platen_max_width: caps.platen_max_width,
            platen_max_height: caps.platen_max_height,
            adf_max_width: caps.adf_max_width,
            adf_max_height: caps.adf_max_height,
            capabilities_xml: caps.raw_xml,
        })
    }

    pub fn start_scan(
        &self,
        requested_source: &str,
        requested_color_mode: &str,
        requested_dpi: u32,
        output_file: &Path,
    ) -> Result<(u32, u32, bool), String> {
        println!("\n[trace] 1. Requesting device capabilities...");
        let caps = self.parse_capabilities()?;
        println!("[trace] Device Firmware Version: {}", caps.version);

        let source = if requested_source.eq_ignore_ascii_case("adf") {
            if caps.adf_max_width.is_some() {
                "Adf"
            } else {
                println!("[trace] ADF not found. Defaulting to Platen.");
                "Platen"
            }
        } else {
            "Platen"
        };

        let resolution = if caps.supported_resolutions.contains(&requested_dpi) {
            requested_dpi
        } else if let Some(&nearest) = caps
            .supported_resolutions
            .iter()
            .min_by_key(|&&dpi| (dpi as i64 - requested_dpi as i64).abs())
        {
            println!(
                "[trace] Negotiated DPI {} to nearest supported: {}",
                requested_dpi, nearest
            );
            nearest
        } else {
            300
        };

        let normalized = requested_color_mode.to_lowercase();
        let color_mode = if normalized.contains("gray") {
            caps.supported_color_modes
                .iter()
                .find(|m| m.eq_ignore_ascii_case("Grayscale8"))
                .cloned()
                .unwrap_or_else(|| "Grayscale8".to_string())
        } else if normalized.contains("black")
            || normalized.contains("mono")
            || normalized.contains("bw")
        {
            caps.supported_color_modes
                .iter()
                .find(|m| m.eq_ignore_ascii_case("BlackAndWhite1"))
                .cloned()
                .unwrap_or_else(|| "BlackAndWhite1".to_string())
        } else {
            caps.supported_color_modes
                .iter()
                .find(|m| m.eq_ignore_ascii_case("RGB24"))
                .cloned()
                .unwrap_or_else(|| "RGB24".to_string())
        };

        let is_gray = color_mode != "RGB24";

        let (region_w, region_h) = if source == "Adf" {
            (
                caps.adf_max_width.unwrap_or(2550),
                caps.adf_max_height.unwrap_or(3508),
            )
        } else {
            (caps.platen_max_width, caps.platen_max_height)
        };

        // Enforce STRICT \r\n (CRLF) line endings. HP firmwares often crash on standard \n.
        let ticket = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n\
<scan:ScanSettings xmlns:scan=\"http://schemas.hp.com/imaging/escl/2011/05/03\" xmlns:pwg=\"http://www.pwg.org/schemas/2010/12/sm\">\r\n\
<pwg:Version>2.63</pwg:Version>\r\n\
<scan:Intent>Document</scan:Intent>\r\n\
<pwg:ScanRegions>\r\n\
<pwg:ScanRegion>\r\n\
<pwg:XOffset>0</pwg:XOffset>\r\n\
<pwg:YOffset>0</pwg:YOffset>\r\n\
<pwg:Width>2550</pwg:Width>\r\n\
<pwg:Height>3508</pwg:Height>\r\n\
</pwg:ScanRegion>\r\n\
</pwg:ScanRegions>\r\n\
<scan:InputSource>Platen</scan:InputSource>\r\n\
<scan:ColorMode>RGB24</scan:ColorMode>\r\n\
<scan:XResolution>300</scan:XResolution>\r\n\
<scan:YResolution>300</scan:YResolution>\r\n\
<scan:DocumentFormat>image/jpeg</scan:DocumentFormat>\r\n\
</scan:ScanSettings>\r\n".to_string(); // Note the trailing \r\n required by some parsers

        let scan_jobs_url = format!("{}/eSCL/ScanJobs", self.base_url);
        println!(
            "\n[trace] 2. Submitting ScanJobs Ticket to {}",
            scan_jobs_url
        );
        println!("[trace] Payload size: {} bytes", ticket.len());

        let resp = self
            .client
            .post(&scan_jobs_url)
            .header("User-Agent", "PostmanRuntime/7.56.1") // Matching working Postman UA
            .header("Accept", "*/*")
            .header("Connection", "keep-alive")
            .header("Content-Type", "text/xml")
            .body(ticket.clone())
            .send()
            .map_err(|e| format!("Network request failed: {}", e))?;

        if !resp.status().is_success() {
            println!("\n[ERROR] Request failed!");
            let status = resp.status();
            println!("HTTP Status: {}", status);
            println!("Response Headers: {:#?}", resp.headers());
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Scanner rejected negotiated ticket.\nStatus: {}\nBody: {}\nSent Ticket:\n{}",
                status, body, ticket
            ));
        }

        let location = resp
            .headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| format!("No Location header returned. Status: {}", resp.status()))?;

        println!(
            "[trace] Scanner accepted job. Location header: {}",
            location
        );

        let next_doc_url = if location.starts_with("http://") || location.starts_with("https://") {
            if let Ok(mut parsed) = reqwest::Url::parse(location) {
                if self.base_url.starts_with("https://") {
                    let _ = parsed.set_scheme("https");
                }
                format!("{}/NextDocument", parsed.as_str().trim_end_matches('/'))
            } else {
                format!("{}/NextDocument", location.trim_end_matches('/'))
            }
        } else {
            let clean_loc = if location.starts_with('/') {
                location.to_string()
            } else {
                format!("/{}", location)
            };
            format!("{}{}/NextDocument", self.base_url, clean_loc)
        };

        println!("[trace] 3. Polling image from: {next_doc_url}");

        let mut final_body = Vec::new();
        let mut attempts = 0;

        while attempts < 30 {
            attempts += 1;
            let doc_resp = self.client.get(&next_doc_url).send();

            match doc_resp {
                Ok(res) if res.status().is_success() => {
                    final_body = res.bytes().map_err(|e| e.to_string())?.to_vec();
                    println!(
                        "[trace] Image payload retrieved ({} bytes).",
                        final_body.len()
                    );
                    break;
                }
                Ok(res) if res.status().as_u16() == 503 || res.status().as_u16() == 409 => {
                    println!(
                        "[trace] Scanner digitizing (HTTP {}), waiting 1s (attempt {attempts}/30)...",
                        res.status()
                    );
                    thread::sleep(Duration::from_millis(1000));
                }
                Ok(res) => {
                    return Err(format!(
                        "Unexpected scanner polling status: {}",
                        res.status()
                    ));
                }
                Err(e) => {
                    println!("[warn] Polling network error: {e}, retrying in 1s...");
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }

        if final_body.is_empty() {
            return Err("Timed out waiting for scanner to deliver image data".to_string());
        }

        std::fs::write(output_file, &final_body)
            .map_err(|e| format!("Failed to save scan: {}", e))?;

        let (px_width, px_height, is_gray_detected) = parse_jpeg_dimensions(&final_body)
            .unwrap_or_else(|| {
                let w = (region_w as f32 / 300.0 * resolution as f32) as u32;
                let h = (region_h as f32 / 300.0 * resolution as f32) as u32;
                (w, h, is_gray)
            });

        Ok((px_width, px_height, is_gray_detected))
    }
}

// =========================================================================
// Tag-Matched XML Parsers
// =========================================================================

pub fn find_tag_content<'a>(xml: &'a str, tag_name: &str) -> Option<&'a str> {
    let open_patterns: Vec<String> = if tag_name.contains(':') {
        vec![format!("<{tag_name}")]
    } else {
        vec![
            format!("<pwg:{tag_name}"),
            format!("<scan:{tag_name}"),
            format!("<{tag_name}"),
        ]
    };

    let mut start_tag_pos = None;
    for pat in &open_patterns {
        if let Some(pos) = xml.find(pat.as_str()) {
            let after = pos + pat.len();
            if after < xml.len() {
                let next_byte = xml.as_bytes()[after];
                if next_byte == b'>'
                    || next_byte == b' '
                    || next_byte == b'\t'
                    || next_byte == b'\n'
                    || next_byte == b'\r'
                    || next_byte == b'/'
                {
                    if start_tag_pos.map_or(true, |p| pos < p) {
                        start_tag_pos = Some(pos);
                    }
                }
            }
        }
    }
    let open_pos = start_tag_pos?;
    let open_tag_end = xml[open_pos..].find('>')? + open_pos + 1;

    let close_patterns: Vec<String> = if tag_name.contains(':') {
        vec![format!("</{tag_name}>")]
    } else {
        vec![
            format!("</pwg:{tag_name}>"),
            format!("</scan:{tag_name}>"),
            format!("</{tag_name}>"),
        ]
    };

    let mut close_pos = None;
    for pat in &close_patterns {
        if let Some(pos) = xml[open_tag_end..].find(pat.as_str()) {
            let abs_pos = open_tag_end + pos;
            if close_pos.map_or(true, |p| abs_pos < p) {
                close_pos = Some(abs_pos);
            }
        }
    }
    let close_pos = close_pos?;
    if close_pos >= open_tag_end {
        Some(xml[open_tag_end..close_pos].trim())
    } else {
        None
    }
}

pub fn find_all_tag_contents(xml: &str, tag_name: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut cursor = 0;

    let open_patterns: Vec<String> = if tag_name.contains(':') {
        vec![format!("<{tag_name}")]
    } else {
        vec![
            format!("<pwg:{tag_name}"),
            format!("<scan:{tag_name}"),
            format!("<{tag_name}"),
        ]
    };
    let close_patterns: Vec<String> = if tag_name.contains(':') {
        vec![format!("</{tag_name}>")]
    } else {
        vec![
            format!("</pwg:{tag_name}>"),
            format!("</scan:{tag_name}>"),
            format!("</{tag_name}>"),
        ]
    };

    while cursor < xml.len() {
        let mut best_open = None;
        for pat in &open_patterns {
            if let Some(rel) = xml[cursor..].find(pat.as_str()) {
                let abs = cursor + rel;
                let after = abs + pat.len();
                if after < xml.len() {
                    let next_byte = xml.as_bytes()[after];
                    if next_byte == b'>'
                        || next_byte == b' '
                        || next_byte == b'\t'
                        || next_byte == b'\n'
                        || next_byte == b'\r'
                    {
                        if best_open.map_or(true, |(p, _)| abs < p) {
                            best_open = Some((abs, pat.as_str()));
                        }
                    }
                }
            }
        }

        let (open_pos, _) = match best_open {
            Some(v) => v,
            None => break,
        };

        let open_tag_end = match xml[open_pos..].find('>') {
            Some(p) => open_pos + p + 1,
            None => break,
        };

        let mut best_close = None;
        for pat in &close_patterns {
            if let Some(rel) = xml[open_tag_end..].find(pat.as_str()) {
                let abs = open_tag_end + rel;
                if best_close.map_or(true, |p| abs < p) {
                    best_close = Some(abs);
                }
            }
        }

        let close_pos = match best_close {
            Some(p) => p,
            None => break,
        };

        let val = xml[open_tag_end..close_pos].trim().to_string();
        if !val.is_empty() {
            results.push(val);
        }
        cursor = close_pos + 1;
    }

    results
}

pub fn parse_jpeg_dimensions(data: &[u8]) -> Option<(u32, u32, bool)> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 9 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 {
            let h = ((data[i + 5] as u32) << 8) | (data[i + 6] as u32);
            let w = ((data[i + 7] as u32) << 8) | (data[i + 8] as u32);
            let components = data[i + 9];
            let is_gray = components == 1;
            return Some((w, h, is_gray));
        }
        if i + 3 < data.len() {
            let len = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
            i += 2 + len;
        } else {
            break;
        }
    }
    None
}

pub fn jpeg_to_pdf(
    jpeg_data: &[u8],
    width_px: u32,
    height_px: u32,
    dpi: u32,
    is_gray: bool,
) -> Vec<u8> {
    let dpi_f = if dpi == 0 { 300.0 } else { dpi as f32 };
    let pt_w = (width_px as f32 / dpi_f * 72.0) as u32;
    let pt_h = (height_px as f32 / dpi_f * 72.0) as u32;

    let color_space = if is_gray { "/DeviceGray" } else { "/DeviceRGB" };
    let content_stream = format!("q {} 0 0 {} 0 0 cm /Im0 Do Q", pt_w, pt_h);

    let mut pdf = format!(
        "%PDF-1.4\n\
        1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
        2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
        3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>\nendobj\n\
        4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
        5 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {} /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        pt_w, pt_h, content_stream.len(), content_stream, width_px, height_px, color_space, jpeg_data.len()
    ).into_bytes();

    pdf.extend_from_slice(jpeg_data);
    pdf.extend_from_slice(b"\nendstream\nendobj\nxref\n0 6\n0000000000 65535 f \ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n%%EOF\n");
    pdf
}
