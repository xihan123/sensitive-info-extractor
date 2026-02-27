use crate::models::MatchInfo;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// 姓名提取 API 请求体
#[derive(Debug, Serialize)]
struct NameExtractRequest {
    text: String,
}

/// 姓名提取 API 响应体
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NameExtractResponse {
    names: Vec<String>,
    confidence: f64,
    #[serde(default)]
    review_id: Option<i64>,
    #[serde(default)]
    is_duplicate: Option<bool>,
}

/// 姓名提取 API 健康检查响应
#[derive(Debug, Deserialize)]
struct HealthResponse {
    #[serde(default)]
    status: String,
}

pub struct NameExtractor {
    client: Client,
    api_host: String,
    enabled: bool,
    /// 失败请求计数器（用于统计）
    failed_count: AtomicUsize,
}

impl NameExtractor {
    pub fn new(api_host: impl Into<String>, enabled: bool) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(5)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            api_host: api_host.into(),
            enabled,
            failed_count: AtomicUsize::new(0),
        }
    }

    /// 获取失败计数
    #[allow(dead_code)]
    pub fn failed_count(&self) -> usize {
        self.failed_count.load(Ordering::Relaxed)
    }

    /// 重置失败计数
    #[allow(dead_code)]
    pub fn reset_failed_count(&self) {
        self.failed_count.store(0, Ordering::Relaxed);
    }

    /// 检查 API 连接状态
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

    /// 从文本中提取姓名
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
    fn test_failed_count() {
        let extractor = NameExtractor::new("localhost:8080", true);
        assert_eq!(extractor.failed_count(), 0);
        extractor.reset_failed_count();
        assert_eq!(extractor.failed_count(), 0);
    }
}
