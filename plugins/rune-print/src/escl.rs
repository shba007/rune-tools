use reqwest::blocking::Client;
use std::path::Path;
use std::thread;
use std::time::Duration;

pub struct EsclClient {
    pub base_url: String,
    client: Client,
}

impl EsclClient {
    pub fn new(printer_ip: &str, port: u16, use_https: bool) -> Self {
        let scheme = if use_https { "https" } else { "http" };
        let base_url = if (use_https && port == 443) || (!use_https && port == 80) {
            format!("{}://{}", scheme, printer_ip)
        } else {
            format!("{}://{}:{}", scheme, printer_ip, port)
        };

        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        Self { base_url, client }
    }

    pub fn get_capabilities(&self) -> Result<String, String> {
        let url = format!("{}/eSCL/ScannerCapabilities", self.base_url);
        let resp = self.client.get(&url).send().map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("HTTP Status {}", resp.status()));
        }

        resp.text().map_err(|e| e.to_string())
    }

    pub fn start_scan(
        &self,
        source: &str,
        color_mode_input: &str,
        resolution: u32,
        output_file: &Path,
    ) -> Result<(u32, u32, bool), String> {
        let (color_mode, is_gray) = match color_mode_input.to_lowercase().as_str() {
            "color" | "rgb" | "rgb24" => ("RGB24", false),
            "grayscale" | "grayscale8" | "gray" => ("Grayscale8", true),
            "blackandwhite" | "blackandwhite1" | "monochrome" | "bw" => ("BlackAndWhite1", true),
            _ => ("RGB24", false),
        };

        // Calculates the exact pixel dimensions based on the requested DPI for PDF generation
        let (px_width, px_height) = match resolution {
            75 => (638, 877),
            100 => (850, 1169),
            150 => (1275, 1754),
            200 => (1700, 2338),
            300 => (2550, 3508),
            400 => (3400, 4677),
            600 => (5100, 7016),
            1200 => (10200, 14032),
            _ => {
                let w = (resolution as f32 * 8.5) as u32;
                let h = (resolution as f32 * 11.69) as u32;
                (w, h)
            }
        };

        // HP firmware strictly requires ScanRegions defined in 300ths of an inch (2550x3300 = 8.5x11)
        let ticket = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ScanSettings xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03" xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm">
    <pwg:Version>2.0</pwg:Version>
    <scan:Intent>Document</scan:Intent>
    <pwg:ScanRegions>
        <pwg:ScanRegion>
            <pwg:XOffset>0</pwg:XOffset>
            <pwg:YOffset>0</pwg:YOffset>
            <pwg:Width>2550</pwg:Width>
            <pwg:Height>3300</pwg:Height>
        </pwg:ScanRegion>
    </pwg:ScanRegions>
    <scan:InputSource>{}</scan:InputSource>
    <scan:ColorMode>{}</scan:ColorMode>
    <scan:XResolution>{}</scan:XResolution>
    <scan:YResolution>{}</scan:YResolution>
    <scan:DocumentFormat>image/jpeg</scan:DocumentFormat>
</scan:ScanSettings>"#,
            source, color_mode, resolution, resolution
        );

        let scan_jobs_url = format!("{}/eSCL/ScanJobs", self.base_url);
        let resp = self
            .client
            .post(&scan_jobs_url)
            .header("Content-Type", "text/xml")
            .body(ticket)
            .send()
            .map_err(|e| format!("Failed to submit Scan Ticket: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Scanner rejected ticket. Status: {}. Body: {}",
                status, body
            ));
        }

        let location = resp
            .headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| format!("No Location header returned. Status: {}", resp.status()))?;

        let next_doc_url = if location.starts_with("http") {
            format!("{}/NextDocument", location.trim_end_matches('/'))
        } else {
            let clean_loc = if location.starts_with('/') {
                location.to_string()
            } else {
                format!("/{}", location)
            };
            format!("{}{}/NextDocument", self.base_url, clean_loc)
        };

        let mut final_body = Vec::new();
        let mut attempts = 0;

        while attempts < 30 {
            attempts += 1;
            let doc_resp = self.client.get(&next_doc_url).send();

            match doc_resp {
                Ok(res) if res.status().is_success() => {
                    final_body = res.bytes().map_err(|e| e.to_string())?.to_vec();
                    break;
                }
                Ok(res) if res.status().as_u16() == 503 || res.status().as_u16() == 409 => {
                    thread::sleep(Duration::from_millis(1000));
                }
                Ok(res) => {
                    return Err(format!("Unexpected scanner status: {}", res.status()));
                }
                Err(_) => thread::sleep(Duration::from_millis(1000)),
            }
        }

        if final_body.is_empty() {
            return Err("Timed out waiting for scanner to deliver image data".to_string());
        }

        std::fs::write(output_file, &final_body)
            .map_err(|e| format!("Failed to save scan: {}", e))?;

        Ok((px_width, px_height, is_gray))
    }
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
