use super::image::{
    generate_preview_image_bytes, list_previewable_images, render_watermarked_image,
    run_batch_image_watermark,
};
use super::pdf::{
    add_text_watermark, extract_embedded_images, generate_pdf_preview_image_bytes,
    list_previewable_pdfs, run_batch_pdf_watermark, split_pdf_to_images,
};
use super::shared::build_slanted_watermark_options;
use super::video::list_previewable_videos;
use crate::models::{
    BatchImageWatermarkInput, BatchImageWatermarkPreviewInput, BatchPdfTextWatermarkInput,
    BatchPdfWatermarkPreviewInput, PdfTextWatermarkInput,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn split_command_rejects_empty_input_path() {
    let err = split_pdf_to_images("".into(), "/tmp".into(), "png".into())
        .expect_err("empty input path should fail");

    assert!(err.contains("PDF 文件"));
}

#[test]
fn watermark_command_rejects_empty_text() {
    let err = add_text_watermark(PdfTextWatermarkInput {
        input_path: "a.pdf".into(),
        output_dir: "/tmp".into(),
        watermark_text: "".into(),
        watermark_long_edge_font_ratio: 0.028,
        watermark_opacity: 50.0 / 255.0,
        watermark_rotation_degrees: -57.29578,
        watermark_stripe_gap_chars: 2.0,
        watermark_row_gap_lines: 3.0,
    })
    .expect_err("empty watermark text should fail");

    assert!(err.contains("水印文字"));
}

#[test]
fn watermark_command_rejects_non_positive_long_edge_font_ratio() {
    let err = add_text_watermark(PdfTextWatermarkInput {
        input_path: "a.pdf".into(),
        output_dir: "/tmp".into(),
        watermark_text: "wm".into(),
        watermark_long_edge_font_ratio: 0.0,
        watermark_opacity: 50.0 / 255.0,
        watermark_rotation_degrees: -57.29578,
        watermark_stripe_gap_chars: 2.0,
        watermark_row_gap_lines: 3.0,
    })
    .expect_err("non-positive long edge font ratio should fail");

    assert!(err.contains("长边字号比例"));
}

#[test]
fn batch_pdf_watermark_command_rejects_same_input_and_output_dir() {
    let err = run_batch_pdf_watermark(
        BatchPdfTextWatermarkInput {
            input_dir: "/tmp/pdfs".into(),
            output_dir: "/tmp/pdfs/".into(),
            watermark_text: "wm".into(),
            watermark_long_edge_font_ratio: 0.028,
            watermark_opacity: 50.0 / 255.0,
            watermark_rotation_degrees: -57.29578,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        },
        |_| {},
    )
    .expect_err("same directories should fail");

    assert!(err.contains("输入目录与输出目录不能相同"));
}

#[test]
fn batch_pdf_watermark_command_skips_existing_nested_output() {
    let temp_dir = TestDir::new();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    let nested_dir = input_dir.join("nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    fs::write(nested_dir.join("demo.pdf"), b"skip me").expect("pdf placeholder should be written");
    let nested_output_dir = output_dir.join("nested");
    fs::create_dir_all(&nested_output_dir).expect("nested output dir should be created");
    fs::write(
        nested_output_dir.join("demo-watermarked.pdf"),
        b"existing output",
    )
    .expect("existing output should be written");

    let result = run_batch_pdf_watermark(
        BatchPdfTextWatermarkInput {
            input_dir: input_dir.to_string_lossy().into_owned(),
            output_dir: output_dir.to_string_lossy().into_owned(),
            watermark_text: "wm".into(),
            watermark_long_edge_font_ratio: 0.028,
            watermark_opacity: 50.0 / 255.0,
            watermark_rotation_degrees: -57.29578,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        },
        |_| {},
    )
    .expect("batch pdf watermark should succeed");

    assert_eq!(result.scanned_file_count, 1);
    assert_eq!(result.success_count, 0);
    assert_eq!(result.failure_count, 0);
    assert_eq!(result.skipped_count, 1);
    assert!(output_dir.join("nested/demo-watermarked.pdf").exists());
}

#[test]
fn batch_pdf_watermark_command_reports_progress() {
    let temp_dir = TestDir::new();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    let nested_dir = input_dir.join("nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    create_test_pdf(&nested_dir.join("a.pdf")).expect("first pdf should be written");
    create_test_pdf(&nested_dir.join("b.pdf")).expect("second pdf should be written");

    let mut progress_events = Vec::new();
    let result = run_batch_pdf_watermark(
        BatchPdfTextWatermarkInput {
            input_dir: input_dir.to_string_lossy().into_owned(),
            output_dir: output_dir.to_string_lossy().into_owned(),
            watermark_text: "wm".into(),
            watermark_long_edge_font_ratio: 0.028,
            watermark_opacity: 50.0 / 255.0,
            watermark_rotation_degrees: -57.29578,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        },
        |progress| progress_events.push(progress),
    )
    .expect("batch pdf watermark should succeed");

    assert_eq!(result.scanned_file_count, 2);
    assert_eq!(result.skipped_count, 0);
    assert_eq!(progress_events.len(), 3);
    assert_eq!(progress_events[0].processed_file_count, 0);
    assert_eq!(progress_events[2].processed_file_count, 2);
}

#[test]
fn batch_pdf_watermark_command_skips_existing_outputs_and_continues_after_failures() {
    let temp_dir = TestDir::new();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    fs::create_dir_all(&input_dir).expect("input dir should be created");
    fs::write(input_dir.join("a.pdf"), b"skip me")
        .expect("first pdf placeholder should be written");
    fs::write(input_dir.join("b.pdf"), b"not a real pdf").expect("invalid pdf should be written");
    fs::write(input_dir.join("c.pdf"), b"still not a real pdf")
        .expect("third pdf placeholder should be written");
    fs::create_dir_all(&output_dir).expect("output dir should be created");
    fs::write(output_dir.join("a-watermarked.pdf"), b"existing")
        .expect("existing output should be written");

    let mut progress_events = Vec::new();
    let result = run_batch_pdf_watermark(
        BatchPdfTextWatermarkInput {
            input_dir: input_dir.to_string_lossy().into_owned(),
            output_dir: output_dir.to_string_lossy().into_owned(),
            watermark_text: "wm".into(),
            watermark_long_edge_font_ratio: 0.028,
            watermark_opacity: 50.0 / 255.0,
            watermark_rotation_degrees: -57.29578,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        },
        |progress| progress_events.push(progress),
    )
    .expect("batch pdf watermark should continue past skips and failures");

    assert_eq!(result.scanned_file_count, 3);
    assert_eq!(result.success_count, 0);
    assert_eq!(result.failure_count, 2);
    assert_eq!(result.skipped_count, 1);
    assert_eq!(
        progress_events.last().map(|it| it.processed_file_count),
        Some(3)
    );
    assert_eq!(progress_events.last().map(|it| it.skipped_count), Some(1));
}

#[test]
fn extract_command_rejects_empty_output_dir() {
    let err = extract_embedded_images("a.pdf".into(), "".into())
        .expect_err("empty output dir should fail");

    assert!(err.contains("输出目录"));
}

#[test]
fn batch_image_watermark_command_rejects_same_input_and_output_dir() {
    let err = run_batch_image_watermark(
        BatchImageWatermarkInput {
            input_dir: "/tmp/images".into(),
            output_dir: "/tmp/images/".into(),
            watermark_text: "wm".into(),
            watermark_line_count: 3,
            watermark_full_screen: true,
            watermark_opacity: 0.2,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        },
        |_| {},
    )
    .expect_err("same directories should fail");

    assert!(err.contains("输入目录与输出目录不能相同"));
}

#[test]
fn batch_image_watermark_command_rejects_invalid_opacity() {
    let err = run_batch_image_watermark(
        BatchImageWatermarkInput {
            input_dir: "/tmp/in".into(),
            output_dir: "/tmp/out".into(),
            watermark_text: "wm".into(),
            watermark_line_count: 3,
            watermark_full_screen: true,
            watermark_opacity: 1.2,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        },
        |_| {},
    )
    .expect_err("opacity greater than one should fail");

    assert!(err.contains("水印透明度"));
}

#[test]
fn batch_image_watermark_command_rejects_negative_spacing() {
    let err = run_batch_image_watermark(
        BatchImageWatermarkInput {
            input_dir: "/tmp/in".into(),
            output_dir: "/tmp/out".into(),
            watermark_text: "wm".into(),
            watermark_line_count: 3,
            watermark_full_screen: true,
            watermark_opacity: 0.2,
            watermark_stripe_gap_chars: -1.0,
            watermark_row_gap_lines: 3.0,
        },
        |_| {},
    )
    .expect_err("negative spacing should fail");

    assert!(err.contains("条间距"));
}

#[test]
fn batch_image_watermark_command_skips_existing_outputs_and_continues_after_failures() {
    let temp_dir = TestDir::new();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    fs::create_dir_all(&input_dir).expect("input dir should be created");
    create_test_png(&input_dir.join("a.png")).expect("first png should be written");
    fs::write(input_dir.join("b.png"), b"not a real png").expect("invalid png should be written");
    create_test_png(&input_dir.join("c.png")).expect("third png should be written");
    fs::create_dir_all(&output_dir).expect("output dir should be created");
    create_test_png(&output_dir.join("a.png")).expect("existing output should be written");

    let mut progress_events = Vec::new();
    let result = run_batch_image_watermark(
        BatchImageWatermarkInput {
            input_dir: input_dir.to_string_lossy().into_owned(),
            output_dir: output_dir.to_string_lossy().into_owned(),
            watermark_text: "wm".into(),
            watermark_line_count: 3,
            watermark_full_screen: true,
            watermark_opacity: 0.2,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        },
        |progress| progress_events.push(progress),
    )
    .expect("batch image watermark should continue past skips and failures");

    assert_eq!(result.scanned_file_count, 3);
    assert_eq!(result.success_count, 1);
    assert_eq!(result.failure_count, 1);
    assert_eq!(result.skipped_count, 1);
    assert!(output_dir.join("c.png").exists());
    assert_eq!(
        progress_events.last().map(|it| it.processed_file_count),
        Some(3)
    );
    assert_eq!(progress_events.last().map(|it| it.skipped_count), Some(1));
}

#[test]
fn build_slanted_watermark_options_preserves_rotation_degrees() {
    let options = build_slanted_watermark_options("wm", 3, true, 0.2, -30.0, 2.0, 3.0)
        .expect("options should be built");

    assert_eq!(options.rotation_degrees, -30.0);
}

#[test]
fn render_watermarked_image_respects_rotation_degrees() {
    let temp_dir = TestDir::new();
    let source_path = temp_dir.path().join("source.png");
    create_test_png(&source_path).expect("png should be written");

    let low_rotation = build_slanted_watermark_options("wm", 3, true, 0.2, -20.0, 2.0, 3.0)
        .expect("low rotation options should build");
    let high_rotation = build_slanted_watermark_options("wm", 3, true, 0.2, -60.0, 2.0, 3.0)
        .expect("high rotation options should build");

    let low = render_watermarked_image(&source_path, &low_rotation)
        .expect("low rotation render should succeed")
        .to_rgba8();
    let high = render_watermarked_image(&source_path, &high_rotation)
        .expect("high rotation render should succeed")
        .to_rgba8();

    assert_ne!(low, high);
}

#[test]
fn list_previewable_images_returns_sorted_relative_image_paths() {
    let temp_dir = TestDir::new();
    let nested_dir = temp_dir.path().join("nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    fs::write(temp_dir.path().join("cover.png"), [1, 2, 3]).expect("png should be written");
    fs::write(nested_dir.join("demo.jpg"), [4, 5, 6]).expect("jpg should be written");
    fs::write(temp_dir.path().join("ignore.txt"), b"noop").expect("txt should be written");

    let files = list_previewable_images(temp_dir.path().to_string_lossy().as_ref())
        .expect("image files should be listed");

    assert_eq!(
        files,
        vec!["cover.png".to_string(), "nested/demo.jpg".to_string()]
    );
}

#[test]
fn list_previewable_videos_returns_sorted_relative_video_paths() {
    let temp_dir = TestDir::new();
    let nested_dir = temp_dir.path().join("nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    fs::write(temp_dir.path().join("cover.mp4"), [1, 2, 3]).expect("mp4 should be written");
    fs::write(nested_dir.join("demo.mov"), [4, 5, 6]).expect("mov should be written");
    fs::write(temp_dir.path().join("ignore.txt"), b"noop").expect("txt should be written");

    let files = list_previewable_videos(temp_dir.path().to_string_lossy().as_ref())
        .expect("video files should be listed");

    assert_eq!(
        files,
        vec!["cover.mp4".to_string(), "nested/demo.mov".to_string()]
    );
}

#[test]
fn list_previewable_pdfs_returns_sorted_relative_pdf_paths() {
    let temp_dir = TestDir::new();
    let nested_dir = temp_dir.path().join("nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    create_test_pdf(&temp_dir.path().join("cover.pdf")).expect("pdf should be written");
    create_test_pdf(&nested_dir.join("demo.pdf")).expect("nested pdf should be written");
    fs::write(temp_dir.path().join("ignore.txt"), b"noop").expect("txt should be written");

    let files =
        list_previewable_pdfs(temp_dir.path().to_string_lossy().as_ref()).expect("pdfs list");

    assert_eq!(
        files,
        vec!["cover.pdf".to_string(), "nested/demo.pdf".to_string()]
    );
}

#[test]
fn generate_preview_image_bytes_rejects_path_escape() {
    let temp_dir = TestDir::new();
    let nested_dir = temp_dir.path().join("nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    fs::write(nested_dir.join("demo.jpg"), [4, 5, 6]).expect("jpg should be written");

    let err = generate_preview_image_bytes(BatchImageWatermarkPreviewInput {
        input_dir: temp_dir.path().to_string_lossy().into_owned(),
        relative_path: "../nested/demo.jpg".into(),
        watermark_text: "wm".into(),
        watermark_line_count: 3,
        watermark_full_screen: true,
        watermark_opacity: 0.2,
        watermark_rotation_degrees: -1.0_f32.to_degrees(),
        watermark_stripe_gap_chars: 2.0,
        watermark_row_gap_lines: 3.0,
    })
    .expect_err("path escape should fail");

    assert!(err.contains("预览图片路径不合法") || err.contains("预览图片不存在"));
}

#[test]
fn generate_pdf_preview_image_bytes_rejects_path_escape() {
    let temp_dir = TestDir::new();
    let nested_dir = temp_dir.path().join("nested");
    fs::create_dir_all(&nested_dir).expect("nested dir should be created");
    create_test_pdf(&nested_dir.join("demo.pdf")).expect("pdf should be written");

    let err = generate_pdf_preview_image_bytes(BatchPdfWatermarkPreviewInput {
        input_dir: temp_dir.path().to_string_lossy().into_owned(),
        relative_path: "../nested/demo.pdf".into(),
        watermark_text: "wm".into(),
        watermark_long_edge_font_ratio: 0.028,
        watermark_opacity: 0.3,
        watermark_rotation_degrees: -35.0,
        watermark_stripe_gap_chars: 2.0,
        watermark_row_gap_lines: 3.0,
    })
    .expect_err("path escape should fail");

    assert!(err.contains("预览 PDF 路径不合法") || err.contains("预览 PDF 不存在"));
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("pdf-split-preview-test-{unique_suffix}-{counter}"));
        fs::create_dir_all(&path).expect("temp dir should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn create_test_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, minimal_pdf_bytes())?;
    Ok(())
}

fn create_test_png(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let image =
        kx_image::RgbaImage::from_pixel(120, 80, kx_image::Rgba([245_u8, 245_u8, 245_u8, 255_u8]));
    image.save(path)?;
    Ok(())
}

fn minimal_pdf_bytes() -> &'static [u8] {
    br#"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Count 1 /Kids [3 0 R] >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 40 >>
stream
BT
/F1 24 Tf
72 72 Td
(Hello PDF) Tj
ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000241 00000 n
0000000330 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
400
%%EOF
"#
}
