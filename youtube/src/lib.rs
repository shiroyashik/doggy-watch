use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub title: String,
}

pub const DEFAULT_YT: &str = "https://youtu.be/";

pub fn extract_youtube_video_id(url: &str) -> Option<String> {
    if let Ok(parsed_url) = Url::parse(url) {
        // Check if the URL is from YouTube or YouTube short URL
        if parsed_url.host_str() == Some("youtube.com") || parsed_url.host_str() == Some("www.youtube.com") || parsed_url.host_str() == Some("music.youtube.com") {
            // Extract the video ID from the query parameters
            if let Some(query) = parsed_url.query() {
                let params: std::collections::HashMap<_, _> = query.split('&')
                    .filter_map(|param| {
                        let (key, value) = param.split_once('=')?;
                        Some((key, value))
                    })
                    .collect();
                if let Some(video_id) = params.get("v") {
                    return Some(video_id.to_string());
                }
            }
        } else if parsed_url.host_str() == Some("youtu.be") {
            // Extract the video ID from the path
            if let Some(mut path) = parsed_url.path_segments() {
                if let Some(video_id) = path.next() {
                    return Some(video_id.to_string());
                }
            }
        }
    }
    None
}

pub async fn get_video_metadata(vid: &str) -> Result<VideoMetadata, reqwest::Error> {
    let response = reqwest::get(format!("https://www.youtube.com/oembed?url={DEFAULT_YT}{vid}")).await?;

    if !response.status().is_success() {
        return Err(response.error_for_status().unwrap_err());
    }

    let mut metadata: VideoMetadata = response.json().await?;

    // ¯\_(ツ)_/¯
    metadata.title = metadata.title.replace("/", "/ ");

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_youtube_video_id_youtu_be() {
        let url = "https://youtu.be/VJFNcHgQ4HM?si=SvWeZZC_UjA1Nhon";
        assert_eq!(extract_youtube_video_id(url), Some("VJFNcHgQ4HM".to_string()));
    }

    #[test]
    fn test_extract_youtube_video_id_youtube_com() {
        let url = "https://www.youtube.com/watch?v=VJFNcHgQ4HM&amp;list=RDCt2h5Xj41Ss&amp;index=2";
        assert_eq!(extract_youtube_video_id(url), Some("VJFNcHgQ4HM".to_string()));
    }

    #[test]
    fn test_extract_youtube_video_id_music_youtube_com() {
        let url = "https://music.youtube.com/watch?v=rfDBTQNdj-M&list=OLAK5uy_nGaGJk4vjvgxE0ff5T9Qus-WEEBYowbBw";
        assert_eq!(extract_youtube_video_id(url), Some("rfDBTQNdj-M".to_string()));
    }

    #[test]
    fn test_extract_youtube_video_id_youtube_com_no_query() {
        let url = "https://www.youtube.com/watch";
        assert_eq!(extract_youtube_video_id(url), None);
    }

    #[test]
    fn test_extract_youtube_video_id_invalid_url() {
        let url = "https://example.com/watch?v=VJFNcHgQ4HM";
        assert_eq!(extract_youtube_video_id(url), None);
    }

    #[test]
    fn test_extract_youtube_video_id_empty_url() {
        let url = "";
        assert_eq!(extract_youtube_video_id(url), None);
    }
}