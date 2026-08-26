use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDownloadInput {
    pub list_path: String,
    pub output_dir: String,
    pub concurrent_downloads: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDownloadSummary {
    pub name: String,
    pub episode_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDownloadListSummary {
    pub episode_count: usize,
    pub series: Vec<SeriesDownloadSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDownloadProgressPayload {
    pub total_count: usize,
    pub processed_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
    pub current_series: Option<String>,
    pub current_episode: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDownloadResult {
    pub total_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
    pub series_count: usize,
    pub output_dir: String,
    pub failed_items: Vec<String>,
}
