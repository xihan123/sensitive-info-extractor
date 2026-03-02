use crate::models::MatchInfo;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub const BATCH_MAX_LIMIT: usize = 64;
pub const BATCH_RECOMMENDED_SIZE: usize = 32;

#[derive(Debug, Serialize)]
struct NameExtractRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
struct NameExtractResponse {
    names: Vec<String>,
    confidence: f64,
}

#[derive(Debug, Serialize)]
struct BatchExtractRequest {
    texts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BatchExtractItem {
    names: Vec<String>,
    confidence: f64,
}

#[derive(Debug, Deserialize)]
struct BatchExtractResponse {
    results: Vec<BatchExtractItem>,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    #[serde(default)]
    status: String,
}

pub struct NameExtractor {
    client: Client,
    api_host: String,
    enabled: bool,
    batch_size: usize,
    failed_count: AtomicUsize,
}

impl NameExtractor {
    pub fn new(api_host: impl Into<String>, enabled: bool) -> Self {
        Self::with_batch_size(api_host, enabled, BATCH_RECOMMENDED_SIZE)
    }

    pub fn with_batch_size(api_host: impl Into<String>, enabled: bool, batch_size: usize) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(5)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            api_host: api_host.into(),
            enabled,
            batch_size: batch_size.min(BATCH_MAX_LIMIT).max(1),
            failed_count: AtomicUsize::new(0),
        }
    }

    pub fn check_connection(&self) -> Result<String, String> {
        let url = format!("http://{}/api/health", self.api_host);

        match self.client.get(&url).timeout(Duration::from_secs(5)).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<HealthResponse>() {
                        Ok(health) => Ok(format!("连接正常: {}", health.status)),
                        Err(_) => Ok("连接正常".to_string()),
                    }
                } else {
                    Err(format!("API 返回状态码: {}", response.status()))
                }
            }
            Err(e) => Err(format!("连接失败: {}", e)),
        }
    }

    pub fn extract(&self, text: &str) -> Vec<MatchInfo> {
        if !self.enabled || text.trim().is_empty() {
            return Vec::new();
        }

        let url = format!("http://{}/api/extract", self.api_host);

        let request = NameExtractRequest {
            text: text.to_string(),
        };

        match self.client.post(&url).json(&request).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<NameExtractResponse>() {
                        Ok(extract_response) => {
                            tracing::debug!(
                                "姓名提取成功: names={:?}, confidence={}",
                                extract_response.names,
                                extract_response.confidence
                            );

                            extract_response
                                .names
                                .into_iter()
                                .map(|name| {
                                    MatchInfo::simple(name, extract_response.confidence >= 0.8)
                                })
                                .collect()
                        }
                        Err(e) => {
                            self.failed_count.fetch_add(1, Ordering::Relaxed);
                            tracing::warn!("解析姓名提取响应失败: {}", e);
                            Vec::new()
                        }
                    }
                } else {
                    self.failed_count.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(
                        "姓名提取 API 返回错误状态: {}",
                        response.status()
                    );
                    Vec::new()
                }
            }
            Err(e) => {
                self.failed_count.fetch_add(1, Ordering::Relaxed);
                tracing::warn!("姓名提取 API 请求失败: {}", e);
                Vec::new()
            }
        }
    }

    pub fn extract_batch(&self, texts: &[&str]) -> Vec<Vec<MatchInfo>> {
        if !self.enabled || texts.is_empty() {
            return vec![Vec::new(); texts.len()];
        }

        let url = format!("http://{}/api/extract/batch", self.api_host);

        let indexed_texts: Vec<(usize, &str)> = texts
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.trim().is_empty())
            .map(|(i, t)| (i, *t))
            .collect();

        if indexed_texts.is_empty() {
            return vec![Vec::new(); texts.len()];
        }

        let mut results: Vec<Vec<MatchInfo>> = vec![Vec::new(); texts.len()];

        for chunk in indexed_texts.chunks(self.batch_size) {
            let texts_to_send: Vec<String> = chunk.iter().map(|(_, t)| t.to_string()).collect();
            let request = BatchExtractRequest { texts: texts_to_send };

            match self.client.post(&url).json(&request).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<BatchExtractResponse>() {
                            Ok(batch_response) => {
                                tracing::debug!(
                                    "批量姓名提取成功: results={}",
                                    batch_response.results.len()
                                );

                                for (i, item) in batch_response.results.into_iter().enumerate() {
                                    if i < chunk.len() {
                                        let original_idx = chunk[i].0;
                                        results[original_idx] = item
                                            .names
                                            .into_iter()
                                            .map(|name| {
                                                MatchInfo::simple(name, item.confidence >= 0.8)
                                            })
                                            .collect();
                                    }
                                }
                            }
                            Err(e) => {
                                self.failed_count.fetch_add(1, Ordering::Relaxed);
                                tracing::warn!("解析批量姓名提取响应失败: {}", e);
                            }
                        }
                    } else {
                        self.failed_count.fetch_add(1, Ordering::Relaxed);
                        tracing::warn!("批量姓名提取 API 返回错误状态: {}", response.status());
                    }
                }
                Err(e) => {
                    self.failed_count.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!("批量姓名提取 API 请求失败: {}", e);
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_extractor_disabled() {
        let extractor = NameExtractor::new("localhost:8080", false);
        let result = extractor.extract("张三和李四参加会议");
        assert!(result.is_empty());
    }

    #[test]
    fn test_name_extractor_empty_text() {
        let extractor = NameExtractor::new("localhost:8080", true);
        let result = extractor.extract("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_batch_size_default() {
        let extractor = NameExtractor::new("localhost:8080", true);
        assert_eq!(extractor.batch_size, BATCH_RECOMMENDED_SIZE);
    }

    #[test]
    fn test_batch_size_custom() {
        let extractor = NameExtractor::with_batch_size("localhost:8080", true, 16);
        assert_eq!(extractor.batch_size, 16);
    }

    #[test]
    fn test_batch_size_limit() {
        let extractor = NameExtractor::with_batch_size("localhost:8080", true, 100);
        assert_eq!(extractor.batch_size, BATCH_MAX_LIMIT);
    }

    #[test]
    fn test_batch_size_minimum() {
        let extractor = NameExtractor::with_batch_size("localhost:8080", true, 0);
        assert_eq!(extractor.batch_size, 1);
    }

    #[test]
    fn test_extract_batch_disabled() {
        let extractor = NameExtractor::new("localhost:8080", false);
        let texts = vec!["张三", "李四"];
        let results = extractor.extract_batch(&texts);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_empty());
        assert!(results[1].is_empty());
    }

    #[test]
    fn test_extract_batch_empty() {
        let extractor = NameExtractor::new("localhost:8080", true);
        let results = extractor.extract_batch(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_extract_batch_all_empty_texts() {
        let extractor = NameExtractor::new("localhost:8080", true);
        let texts = vec!["", "   ", ""];
        let results = extractor.extract_batch(&texts);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_empty()));
    }

    #[test]
    fn test_constants() {
        assert_eq!(BATCH_MAX_LIMIT, 64);
        assert_eq!(BATCH_RECOMMENDED_SIZE, 32);
    }
}
