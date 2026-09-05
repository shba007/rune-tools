#[cfg(target_arch = "wasm32")]
use crate::types::{CmdExecRequest, CmdExecResponse};
#[cfg(target_arch = "wasm32")]
use rune_pdk::ToolCallRequest;
#[cfg(target_arch = "wasm32")]
use serde_json::{Value, json};

#[cfg(target_arch = "wasm32")]
#[extism_pdk::host_fn("extism:host/user")]
extern "ExtismHost" {
    fn host_cmd_exec(input: String) -> String;
}

#[cfg(target_arch = "wasm32")]
pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let payload_str =
        serde_json::to_string(&request).map_err(|e| format!("Serialization error: {}", e))?;

    let cmd_req = CmdExecRequest {
        program: "rune-slides-native".to_string(),
        args: vec!["--exec".to_string(), payload_str],
        cwd: None,
    };

    let raw_req = serde_json::to_string(&cmd_req).map_err(|e| e.to_string())?;
    let raw_resp =
        unsafe { host_cmd_exec(raw_req) }.map_err(|e| format!("Host execution failed: {:?}", e))?;

    let resp: CmdExecResponse = serde_json::from_str(&raw_resp)
        .map_err(|e| format!("Failed to parse host response: {}", e))?;

    if !resp.success && resp.stdout.trim().is_empty() {
        return Err(if !resp.stderr.is_empty() {
            resp.stderr
        } else {
            "rune-slides-native exited with failure".to_string()
        });
    }

    let parsed_val: Value = serde_json::from_str(&resp.stdout).map_err(|e| {
        format!(
            "Failed to parse output JSON: {} (stdout: {})",
            e, resp.stdout
        )
    })?;

    if let Some(err) = parsed_val.get("error").and_then(Value::as_str) {
        return Err(err.to_string());
    }

    Ok(parsed_val)
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use crate::types::{SlidePage, SlideProject};
    use printpdf::{Base64OrRaw, GeneratePdfOptions, PdfDocument, PdfSaveOptions, PdfWarnMsg};
    use rune_pdk::ToolCallRequest;
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use zip::write::SimpleFileOptions;

    fn get_config(key: &str) -> Option<String> {
        std::env::var(key.to_ascii_uppercase())
            .or_else(|_| std::env::var(key))
            .ok()
            .filter(|s| !s.is_empty())
    }

    pub fn resolve_path(path_param: Option<&str>, default_filename: &str) -> PathBuf {
        let raw = path_param
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(default_filename);

        let target = PathBuf::from(raw);
        let allowed_root = get_config("allowed_dir").or_else(|| get_config("OUTPUT_DIR"));

        if let Some(root_str) = allowed_root {
            let root = PathBuf::from(root_str);
            if target.is_relative() {
                root.join(target)
            } else {
                target
            }
        } else {
            target
        }
    }

    fn load_project(path: &Path) -> Result<SlideProject, String> {
        if !path.exists() {
            return Err(format!(
                "Presentation project not found at '{}'. Run slide_init first.",
                path.display()
            ));
        }
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse project JSON: {}", e))
    }

    fn save_project(path: &Path, project: &SlideProject) -> Result<(), String> {
        let data = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| format!("Failed to write project file: {}", e))
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    fn export_to_pptx(project: &SlideProject, output_path: &Path) -> Result<(), String> {
        let file = fs::File::create(output_path)
            .map_err(|e| format!("Failed to create PPTX file: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut ct_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
"#,
        );
        for i in 1..=project.slides.len() {
            ct_xml.push_str(&format!(
                r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
                i
            ));
        }
        ct_xml.push_str("</Types>");
        zip.start_file("[Content_Types].xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(ct_xml.as_bytes())
            .map_err(|e| e.to_string())?;

        zip.start_file("_rels/.rels", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#,
        )
        .map_err(|e| e.to_string())?;

        zip.start_file("docProps/core.xml", options)
            .map_err(|e| e.to_string())?;
        let core_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{}</dc:title><cp:revision>1</cp:revision></cp:coreProperties>"#,
            escape_xml(&project.title)
        );
        zip.write_all(core_xml.as_bytes())
            .map_err(|e| e.to_string())?;

        zip.start_file("docProps/app.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Rune Slides</Application></Properties>"#,
        )
        .map_err(|e| e.to_string())?;

        let mut pres_rels = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
"#,
        );
        for i in 1..=project.slides.len() {
            pres_rels.push_str(&format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                i + 2,
                i
            ));
        }
        pres_rels.push_str("</Relationships>");
        zip.start_file("ppt/_rels/presentation.xml.rels", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(pres_rels.as_bytes())
            .map_err(|e| e.to_string())?;

        let mut pres_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst>
"#,
        );
        for i in 1..=project.slides.len() {
            pres_xml.push_str(&format!(
                r#"<p:sldId id="{}" r:id="rId{}"/>"#,
                255 + i,
                i + 2
            ));
        }
        pres_xml.push_str(
            r#"</p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#,
        );
        zip.start_file("ppt/presentation.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(pres_xml.as_bytes())
            .map_err(|e| e.to_string())?;

        zip.start_file("ppt/theme/theme1.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Arial"/></a:majorFont><a:minorFont><a:latin typeface="Arial"/></a:minorFont></a:fontScheme><a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectLst/></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#).map_err(|e| e.to_string())?;

        zip.start_file("ppt/slideMasters/slideMaster1.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst></p:sldMaster>"#).map_err(|e| e.to_string())?;

        zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#).map_err(|e| e.to_string())?;

        zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld></p:sldLayout>"#).map_err(|e| e.to_string())?;

        zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#).map_err(|e| e.to_string())?;

        for (idx, slide) in project.slides.iter().enumerate() {
            let slide_filename = format!("ppt/slides/slide{}.xml", idx + 1);
            zip.start_file(&slide_filename, options)
                .map_err(|e| e.to_string())?;

            let slide_body = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="685800" y="685800"/><a:ext cx="10820400" cy="1143000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz="4400"/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Content"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="685800" y="2133600"/><a:ext cx="10820400" cy="4038600"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz="2400"/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld>
</p:sld>"#,
                escape_xml(&slide.title),
                escape_xml(&slide.content)
            );
            zip.write_all(slide_body.as_bytes())
                .map_err(|e| e.to_string())?;

            let slide_rel_filename = format!("ppt/slides/_rels/slide{}.xml.rels", idx + 1);
            zip.start_file(&slide_rel_filename, options)
                .map_err(|e| e.to_string())?;
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#).map_err(|e| e.to_string())?;
        }

        zip.finish().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn export_to_pdf(project: &SlideProject, output_path: &Path) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        let font_path = "C:\\Windows\\Fonts\\arial.ttf";
        #[cfg(target_os = "macos")]
        let font_path = "/System/Library/Fonts/Supplemental/Arial.ttf";
        #[cfg(target_os = "linux")]
        let font_path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

        let font_bytes = fs::read(font_path)
            .map_err(|_| format!("Could not load OS layout font at: {}", font_path))?;

        let mut fonts = BTreeMap::new();
        fonts.insert("sans-serif".to_string(), Base64OrRaw::Raw(font_bytes));

        let mut html = r#"<!DOCTYPE html><html><head><style>
            body { font-family: "sans-serif"; font-size: 24px; padding: 40px; background-color: #ffffff; color: #000000; }
            h1 { font-size: 48px; color: #333333; margin-bottom: 20px; border-bottom: 2px solid #ccc; }
            p { line-height: 1.6; }
            .slide { page-break-after: always; height: 100vh; }
            </style></head><body>"#.to_string();

        for slide in &project.slides {
            html.push_str(&format!(
                r#"<div class="slide"><h1>{}</h1><p>{}</p></div>"#,
                escape_xml(&slide.title),
                escape_xml(&slide.content).replace("\n", "<br/>")
            ));
        }
        html.push_str("</body></html>");

        let mut warnings: Vec<PdfWarnMsg> = Vec::new();
        let doc = PdfDocument::from_html(
            &html,
            &BTreeMap::new(),
            &fonts,
            &GeneratePdfOptions::default(),
            &mut warnings,
        )
        .map_err(|e| format!("PDF layout compilation error: {:?}", e))?;

        let pdf_bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
        fs::write(output_path, pdf_bytes).map_err(|e| format!("Failed to write PDF: {}", e))?;
        Ok(())
    }

    pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
        let args = &request.arguments;

        match request.name.as_str() {
            "slide_init" => {
                let path_arg = args.get("projectPath").and_then(Value::as_str);
                let project_path = resolve_path(path_arg, "presentation.json");

                let title = args
                    .get("title")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'title'")?;
                let theme = args
                    .get("themeName")
                    .and_then(Value::as_str)
                    .unwrap_or("modern");

                let project = SlideProject {
                    title: title.to_string(),
                    theme: theme.to_string(),
                    slides: vec![SlidePage {
                        title: title.to_string(),
                        content: format!("Welcome to {}", title),
                        layout: Some("center".to_string()),
                    }],
                };

                if let Some(parent) = project_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                save_project(&project_path, &project)?;

                Ok(json!({
                    "status": "success",
                    "projectPath": project_path.to_string_lossy(),
                    "message": format!("Initialized presentation '{}'", title)
                }))
            }

            "slide_update_theme" => {
                let path_arg = args.get("projectPath").and_then(Value::as_str);
                let project_path = resolve_path(path_arg, "presentation.json");

                let theme_def = args
                    .get("themeDefinition")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'themeDefinition'")?;

                let theme_path = project_path.with_extension("theme.md");
                fs::write(&theme_path, theme_def).map_err(|e| e.to_string())?;

                Ok(json!({
                    "status": "success",
                    "themePath": theme_path.to_string_lossy(),
                    "message": "Central theme definition updated."
                }))
            }

            "slide_add_page" => {
                let path_arg = args.get("projectPath").and_then(Value::as_str);
                let project_path = resolve_path(path_arg, "presentation.json");

                let slide_title = args
                    .get("slideTitle")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'slideTitle'")?;
                let content = args
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or("Missing 'content'")?;
                let layout = args
                    .get("layout")
                    .and_then(Value::as_str)
                    .map(ToString::to_string);
                let index = args
                    .get("index")
                    .and_then(Value::as_u64)
                    .map(|v| v as usize);

                let mut project = load_project(&project_path)?;

                let new_page = SlidePage {
                    title: slide_title.to_string(),
                    content: content.to_string(),
                    layout,
                };

                if let Some(idx) = index {
                    if idx <= project.slides.len() {
                        project.slides.insert(idx, new_page);
                    } else {
                        project.slides.push(new_page);
                    }
                } else {
                    project.slides.push(new_page);
                }

                save_project(&project_path, &project)?;
                Ok(json!({ "status": "success", "totalSlides": project.slides.len() }))
            }

            "slide_delete_page" => {
                let path_arg = args.get("projectPath").and_then(Value::as_str);
                let project_path = resolve_path(path_arg, "presentation.json");

                let index = args
                    .get("index")
                    .and_then(Value::as_u64)
                    .ok_or("Missing 'index'")? as usize;

                let mut project = load_project(&project_path)?;

                if index >= project.slides.len() {
                    return Err(format!("Index {} out of bounds", index));
                }
                project.slides.remove(index);
                save_project(&project_path, &project)?;

                Ok(json!({ "status": "success", "remainingSlides": project.slides.len() }))
            }

            "slide_export" => {
                let path_arg = args.get("projectPath").and_then(Value::as_str);
                let project_path = resolve_path(path_arg, "presentation.json");

                let out_arg = args.get("outputPath").and_then(Value::as_str);
                let format = args.get("format").and_then(Value::as_str).unwrap_or("pptx");

                let default_out = format!("presentation.{}", format);
                let output_path = resolve_path(out_arg, &default_out);
                let project = load_project(&project_path)?;

                if format.eq_ignore_ascii_case("pdf") {
                    export_to_pdf(&project, &output_path)?;
                    Ok(json!({
                        "status": "success",
                        "format": "pdf",
                        "outputPath": output_path.to_string_lossy(),
                        "message": "Successfully exported to structured vector PDF (.pdf)"
                    }))
                } else {
                    export_to_pptx(&project, &output_path)?;
                    Ok(json!({
                        "status": "success",
                        "format": "pptx",
                        "outputPath": output_path.to_string_lossy(),
                        "message": "Successfully exported to native PowerPoint (.pptx)"
                    }))
                }
            }

            unknown => Err(format!("Unknown tool: {}", unknown)),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::execute_tool;
