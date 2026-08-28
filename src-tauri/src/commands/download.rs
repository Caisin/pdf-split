use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tauri::{Emitter, EventTarget, Manager, WebviewWindow};

use crate::models::{
    SeriesDownloadInput, SeriesDownloadListSummary, SeriesDownloadProgressPayload,
    SeriesDownloadResult, SeriesDownloadSummary,
};

use super::shared::{absolutize_path, require_value};

const SERIES_DOWNLOAD_PROGRESS_EVENT: &str = "series-download-progress";
const MAX_CONCURRENT_DOWNLOADS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadItem {
    url: String,
    series_name: String,
    directory_name: String,
    episode: usize,
}

#[derive(Debug)]
enum DownloadStatus {
    Success,
    Failure(String),
    Skipped,
}

#[derive(Debug)]
struct DownloadOutcome {
    item: DownloadItem,
    status: DownloadStatus,
}

pub fn inspect_series_download_directory(
    input_dir: String,
) -> Result<SeriesDownloadListSummary, String> {
    let (items, file_count) = read_download_directory(&input_dir)?;
    Ok(build_summary(&items, file_count))
}

pub async fn download_series_videos(
    window: WebviewWindow,
    payload: SeriesDownloadInput,
) -> Result<SeriesDownloadResult, String> {
    let window_label = window.label().to_string();
    let app_handle = window.app_handle().clone();

    run_series_download(payload, |progress| {
        let _ = app_handle.emit_to(
            EventTarget::webview_window(window_label.clone()),
            SERIES_DOWNLOAD_PROGRESS_EVENT,
            progress,
        );
    })
    .await
}

async fn run_series_download<F>(
    payload: SeriesDownloadInput,
    mut on_progress: F,
) -> Result<SeriesDownloadResult, String>
where
    F: FnMut(SeriesDownloadProgressPayload),
{
    let input_dir = require_value("TXT 清单目录", payload.input_dir)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    if !(1..=MAX_CONCURRENT_DOWNLOADS).contains(&payload.concurrent_downloads) {
        return Err(format!(
            "同时下载数必须在 1 到 {MAX_CONCURRENT_DOWNLOADS} 之间"
        ));
    }

    let (items, _) = read_download_directory(&input_dir)?;
    let output_root = absolutize_path(Path::new(&output_dir))?;
    fs::create_dir_all(&output_root).map_err(|error| format!("无法创建输出目录：{error}"))?;
    let output_root = output_root
        .canonicalize()
        .map_err(|error| format!("无法访问输出目录：{error}"))?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .user_agent("pdf-split-series-downloader/0.1")
        .build()
        .map_err(|error| format!("无法创建下载客户端：{error}"))?;

    let total_count = items.len();
    let series_count = items
        .iter()
        .map(|item| item.directory_name.as_str())
        .collect::<HashSet<_>>()
        .len();
    on_progress(SeriesDownloadProgressPayload {
        total_count,
        processed_count: 0,
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        current_series: None,
        current_episode: None,
    });

    let items = Arc::new(items);
    let next_index = Arc::new(AtomicUsize::new(0));
    let worker_count = payload.concurrent_downloads.min(total_count);
    let (sender, mut receiver) = tauri::async_runtime::channel::<DownloadOutcome>(worker_count);
    let mut result = SeriesDownloadResult {
        total_count,
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        series_count,
        output_dir: output_root.to_string_lossy().into_owned(),
        failed_items: Vec::new(),
    };

    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let client = client.clone();
        let items = Arc::clone(&items);
        let sender = sender.clone();
        let output_root = output_root.clone();
        let next_index = Arc::clone(&next_index);
        workers.push(tauri::async_runtime::spawn(async move {
            loop {
                let index = next_index.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(index).cloned() else {
                    break;
                };
                let status = download_item(&client, &output_root, &item).await;
                if sender.send(DownloadOutcome { item, status }).await.is_err() {
                    break;
                }
            }
        }));
    }
    drop(sender);

    for _ in 0..total_count {
        let outcome = receiver
            .recv()
            .await
            .ok_or_else(|| "下载任务意外中断".to_string())?;
        match outcome.status {
            DownloadStatus::Success => result.success_count += 1,
            DownloadStatus::Skipped => result.skipped_count += 1,
            DownloadStatus::Failure(reason) => {
                result.failure_count += 1;
                result.failed_items.push(format!(
                    "{} 第{}集：{}",
                    outcome.item.series_name, outcome.item.episode, reason
                ));
            }
        }

        on_progress(SeriesDownloadProgressPayload {
            total_count,
            processed_count: result.success_count + result.failure_count + result.skipped_count,
            success_count: result.success_count,
            failure_count: result.failure_count,
            skipped_count: result.skipped_count,
            current_series: Some(outcome.item.series_name),
            current_episode: Some(outcome.item.episode),
        });
    }

    for worker in workers {
        worker
            .await
            .map_err(|error| format!("下载任务意外中断：{error}"))?;
    }

    Ok(result)
}

async fn download_item(
    client: &reqwest::Client,
    output_root: &Path,
    item: &DownloadItem,
) -> DownloadStatus {
    let series_dir = output_root.join(&item.directory_name);
    if let Err(error) = fs::create_dir_all(&series_dir) {
        return DownloadStatus::Failure(format!("无法创建剧名目录：{error}"));
    }

    let output_path = series_dir.join(format!("{}.mp4", item.episode));
    if output_path.is_file() {
        return DownloadStatus::Skipped;
    }

    let temporary_path = series_dir.join(format!("{}.mp4.part", item.episode));
    let mut last_error = String::new();
    for _ in 0..=2 {
        let _ = fs::remove_file(&temporary_path);
        match download_response_to_file(client, &item.url, &temporary_path).await {
            Ok(()) => {
                return match fs::rename(&temporary_path, &output_path) {
                    Ok(()) => DownloadStatus::Success,
                    Err(error) => {
                        let _ = fs::remove_file(&temporary_path);
                        DownloadStatus::Failure(format!("无法保存文件：{error}"))
                    }
                };
            }
            Err(error) => last_error = error,
        }
    }

    let _ = fs::remove_file(&temporary_path);
    DownloadStatus::Failure(last_error)
}

async fn download_response_to_file(
    client: &reqwest::Client,
    url: &str,
    temporary_path: &Path,
) -> Result<(), String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("请求失败：{error}"))?
        .error_for_status()
        .map_err(|error| format!("服务器返回错误：{error}"))?;
    let mut file =
        fs::File::create(temporary_path).map_err(|error| format!("无法创建临时文件：{error}"))?;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("读取响应失败：{error}"))?
    {
        file.write_all(&chunk)
            .map_err(|error| format!("写入文件失败：{error}"))?;
    }
    file.flush()
        .map_err(|error| format!("写入文件失败：{error}"))?;
    Ok(())
}

fn read_download_directory(input_dir: &str) -> Result<(Vec<DownloadItem>, usize), String> {
    let root = PathBuf::from(input_dir);
    if !root.is_dir() {
        return Err("TXT 清单目录不存在或不是有效目录".to_string());
    }

    let mut text_files = Vec::new();
    collect_text_files(&root, &mut text_files)?;
    text_files.sort();
    if text_files.is_empty() {
        return Err("所选目录中没有 TXT 下载清单".to_string());
    }

    let mut items = Vec::new();
    let mut identities = HashSet::new();
    for path in &text_files {
        let relative_path = path.strip_prefix(&root).unwrap_or(path.as_path());
        let content = fs::read_to_string(path)
            .map_err(|error| format!("无法读取 TXT 清单“{}”：{error}", relative_path.display()))?;
        let file_items = parse_download_list(&content)
            .map_err(|error| format!("TXT 清单“{}”解析失败：{error}", relative_path.display()))?;

        for item in file_items {
            let identity = (item.directory_name.clone(), item.episode);
            if !identities.insert(identity) {
                return Err(format!(
                    "TXT 清单“{}”存在跨文件重复剧集：{} 第{}集",
                    relative_path.display(),
                    item.series_name,
                    item.episode
                ));
            }
            items.push(item);
        }
    }

    items.sort_by(|left, right| {
        left.series_name
            .cmp(&right.series_name)
            .then(left.episode.cmp(&right.episode))
    });
    Ok((items, text_files.len()))
}

fn collect_text_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|error| format!("无法读取 TXT 清单目录“{}”：{error}", dir.display()))?
    {
        let entry = entry.map_err(|error| format!("无法读取目录项：{error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("无法读取文件类型：{error}"))?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_text_files(&path, files)?;
        } else if file_type.is_file() && is_text_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
}

fn parse_download_list(content: &str) -> Result<Vec<DownloadItem>, String> {
    let mut items = Vec::new();
    let mut identities = HashSet::new();

    for (line_index, raw_line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }

        let columns: Vec<&str> = line.split(',').map(str::trim).collect();
        let url_index = columns
            .iter()
            .position(|column| column.starts_with("https://") || column.starts_with("http://"))
            .ok_or_else(|| format!("第 {line_number} 行缺少 http/https 视频地址"))?;
        let url = columns[url_index];
        let title = columns
            .get(url_index + 1)
            .copied()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("第 {line_number} 行缺少剧名和集数"))?;
        let (series_name, episode) = parse_episode_title(title)
            .ok_or_else(|| format!("第 {line_number} 行标题格式应为“剧名 - 第N集”"))?;
        let directory_name = sanitize_directory_name(&series_name)
            .ok_or_else(|| format!("第 {line_number} 行剧名不能用作目录名"))?;
        let identity = (directory_name.clone(), episode);
        if !identities.insert(identity) {
            return Err(format!(
                "第 {line_number} 行存在重复剧集：{series_name} 第{episode}集"
            ));
        }

        items.push(DownloadItem {
            url: url.to_string(),
            series_name,
            directory_name,
            episode,
        });
    }

    if items.is_empty() {
        return Err("下载清单中没有可下载的剧集".to_string());
    }
    items.sort_by(|left, right| {
        left.series_name
            .cmp(&right.series_name)
            .then(left.episode.cmp(&right.episode))
    });
    Ok(items)
}

fn parse_episode_title(title: &str) -> Option<(String, usize)> {
    let marker = " - 第";
    let marker_index = title.rfind(marker)?;
    let series_name = title[..marker_index].trim();
    let episode_text = title[marker_index + marker.len()..]
        .trim()
        .strip_suffix('集')?
        .trim();
    let episode = episode_text.parse::<usize>().ok()?;
    if series_name.is_empty() || episode == 0 {
        return None;
    }
    Some((series_name.to_string(), episode))
}

fn sanitize_directory_name(name: &str) -> Option<String> {
    let sanitized: String = name
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let sanitized = sanitized.trim().trim_end_matches(['.', ' ']).trim();
    if sanitized.is_empty() || matches!(sanitized, "." | "..") {
        None
    } else {
        Some(sanitized.to_string())
    }
}

fn build_summary(items: &[DownloadItem], file_count: usize) -> SeriesDownloadListSummary {
    let mut counts = BTreeMap::<String, usize>::new();
    for item in items {
        *counts.entry(item.series_name.clone()).or_default() += 1;
    }
    SeriesDownloadListSummary {
        file_count,
        episode_count: items.len(),
        series: counts
            .into_iter()
            .map(|(name, episode_count)| SeriesDownloadSummary {
                name,
                episode_count,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn parses_url_pull_lines_and_builds_numeric_output_names() {
        let items = parse_download_list(
            ",https://example.com/a.mp4,生死线倒计时 - 第1集\n\
             ,https://example.com/b.mp4,生死线倒计时 - 第2集",
        )
        .expect("valid list should parse");

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].series_name, "生死线倒计时");
        assert_eq!(items[0].directory_name, "生死线倒计时");
        assert_eq!(items[0].episode, 1);
        assert_eq!(format!("{}.mp4", items[1].episode), "2.mp4");
    }

    #[test]
    fn supports_multiple_series_and_sanitizes_directory_names() {
        let items = parse_download_list(
            "https://example.com/a.mp4,剧/名 - 第1集\n\
             https://example.com/b.mp4,另一部剧 - 第3集",
        )
        .expect("valid list should parse");
        let summary = build_summary(&items, 2);

        assert_eq!(
            items
                .iter()
                .find(|item| item.series_name == "剧/名")
                .map(|item| item.directory_name.as_str()),
            Some("剧_名")
        );
        assert_eq!(summary.episode_count, 2);
        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.series.len(), 2);
    }

    #[test]
    fn reads_all_txt_files_recursively_and_ignores_other_extensions() {
        let root = test_directory("download-directory");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("nested directory should be created");
        fs::write(
            root.join("first.txt"),
            "https://example.com/1.mp4,第一部剧 - 第1集",
        )
        .expect("first list should be written");
        fs::write(
            nested.join("second.TXT"),
            "https://example.com/2.mp4,第二部剧 - 第2集",
        )
        .expect("second list should be written");
        fs::write(
            root.join("ignored.csv"),
            "https://example.com/3.mp4,不应导入 - 第3集",
        )
        .expect("ignored file should be written");

        let (items, file_count) =
            read_download_directory(&root.to_string_lossy()).expect("directory lists should parse");
        let summary = build_summary(&items, file_count);

        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.episode_count, 2);
        assert_eq!(summary.series.len(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_duplicate_episode_targets_across_txt_files() {
        let root = test_directory("download-duplicate");
        fs::create_dir_all(&root).expect("test directory should be created");
        fs::write(
            root.join("first.txt"),
            "https://example.com/1.mp4,同一部剧 - 第1集",
        )
        .expect("first list should be written");
        fs::write(
            root.join("second.txt"),
            "https://example.com/2.mp4,同一部剧 - 第1集",
        )
        .expect("second list should be written");

        let error = read_download_directory(&root.to_string_lossy())
            .expect_err("duplicates across files should fail");

        assert!(error.contains("跨文件重复剧集"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_duplicate_episode_targets() {
        let error = parse_download_list(
            "https://example.com/a.mp4,同一部剧 - 第1集\n\
             https://example.com/b.mp4,同一部剧 - 第1集",
        )
        .expect_err("duplicates should fail");

        assert!(error.contains("重复剧集"));
    }

    #[test]
    fn rejects_invalid_title_format() {
        let error = parse_download_list("https://example.com/a.mp4,没有集数")
            .expect_err("invalid title should fail");

        assert!(error.contains("剧名 - 第N集"));
    }

    #[test]
    fn rejects_concurrency_outside_the_supported_range() {
        let error = tauri::async_runtime::block_on(run_series_download(
            SeriesDownloadInput {
                input_dir: "lists".into(),
                output_dir: "/tmp".into(),
                concurrent_downloads: 21,
            },
            |_| {},
        ))
        .expect_err("excessive concurrency should fail before reading the file");

        assert!(error.contains("1 到 20"));
    }

    #[test]
    fn downloads_directory_lists_into_series_directories() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server should have address");
        let server = std::thread::spawn(move || {
            for (index, stream) in listener.incoming().take(2).enumerate() {
                let mut stream = stream.expect("test request should connect");
                let mut request = [0_u8; 1024];
                let _ = stream.read(&mut request);
                let body = format!("video-{}", index + 1);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("test response should be sent");
            }
        });

        let root = test_directory("download-test");
        let input_dir = root.join("lists");
        let output_dir = root.join("output");
        fs::create_dir_all(&input_dir).expect("input directory should be created");
        fs::write(
            input_dir.join("first.txt"),
            format!(",http://{address}/1.mp4,第一部剧 - 第1集"),
        )
        .expect("first download list should be written");
        fs::write(
            input_dir.join("second.txt"),
            format!(",http://{address}/2.mp4,第二部剧 - 第2集"),
        )
        .expect("second download list should be written");

        let mut progress = Vec::new();
        let result = tauri::async_runtime::block_on(run_series_download(
            SeriesDownloadInput {
                input_dir: input_dir.to_string_lossy().into_owned(),
                output_dir: output_dir.to_string_lossy().into_owned(),
                concurrent_downloads: 2,
            },
            |event| progress.push(event),
        ))
        .expect("series download should succeed");

        server.join().expect("test server should finish");
        assert_eq!(result.success_count, 2);
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.series_count, 2);
        assert!(output_dir.join("第一部剧/1.mp4").is_file());
        assert!(output_dir.join("第二部剧/2.mp4").is_file());
        assert!(!output_dir.join("第一部剧/1.mp4.part").exists());
        assert_eq!(progress.last().map(|event| event.processed_count), Some(2));
        let _ = fs::remove_dir_all(root);
    }

    fn test_directory(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("pdf-split-{label}-{unique}"))
    }
}
