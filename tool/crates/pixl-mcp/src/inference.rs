/// Local inference via mlx_lm.server — manages the sidecar process and
/// sends generation requests with the trained LoRA adapter.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

/// Configuration for the local mlx_lm inference server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Base model ID (e.g. "mlx-community/Qwen2.5-3B-Instruct-4bit")
    pub model: String,
    /// Path to the LoRA adapter directory (safetensors format)
    pub adapter_path: Option<PathBuf>,
    /// Port for the mlx_lm.server process (default: 8099)
    pub port: u16,
    /// Max tokens to generate per request
    pub max_tokens: u32,
    /// Sampling temperature
    pub temperature: f32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            model: "mlx-community/Qwen2.5-3B-Instruct-4bit".to_string(),
            adapter_path: None,
            port: 8099,
            max_tokens: 512,
            temperature: 0.7,
        }
    }
}

impl InferenceConfig {
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url())
    }

    pub fn health_url(&self) -> String {
        format!("{}/v1/models", self.base_url())
    }
}

/// Manages the mlx_lm.server sidecar process.
pub struct InferenceServer {
    pub config: InferenceConfig,
    child: Option<Child>,
}

impl InferenceServer {
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            config,
            child: None,
        }
    }

    /// Check if the server is reachable.
    pub async fn is_healthy(&self) -> bool {
        let url = self.config.health_url();
        match reqwest::Client::new()
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Spawn the mlx_lm.server process if not already running.
    /// Returns Ok(true) if spawned, Ok(false) if already running.
    pub async fn ensure_running(&mut self) -> Result<bool, String> {
        if self.is_healthy().await {
            return Ok(false);
        }

        // Kill stale child if it exists
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            self.child = None;
        }

        eprintln!(
            "starting mlx_lm.server on port {} with model {}",
            self.config.port, self.config.model
        );

        let mut cmd = Command::new("python");
        cmd.arg("-m")
            .arg("mlx_lm.server")
            .arg("--model")
            .arg(&self.config.model)
            .arg("--port")
            .arg(self.config.port.to_string());

        let child = cmd.spawn().map_err(|e| {
            format!(
                "failed to start mlx_lm.server: {}. Is mlx-lm installed? (pip install mlx-lm)",
                e
            )
        })?;
        self.child = Some(child);

        // Wait for server to become healthy (up to 30s for model loading)
        for i in 0..60 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if self.is_healthy().await {
                eprintln!("mlx_lm.server ready after {}ms", (i + 1) * 500);
                return Ok(true);
            }
        }

        Err("mlx_lm.server failed to start within 30s".to_string())
    }

    /// Send a chat completion request to the local model.
    pub async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let messages = vec![
            serde_json::json!({"role": "system", "content": system_prompt}),
            serde_json::json!({"role": "user", "content": user_prompt}),
        ];

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        // Attach adapter path if configured
        if let Some(ref adapter) = self.config.adapter_path {
            body["adapters"] = serde_json::json!(adapter.to_string_lossy());
        }

        let resp = reqwest::Client::new()
            .post(&self.config.completions_url())
            .json(&body)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("inference request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("inference server returned {}: {}", status, text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("failed to parse inference response: {}", e))?;

        // Extract the assistant message content from OpenAI-compatible response
        json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                format!(
                    "unexpected response format: {}",
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                )
            })
    }

    /// Stop the sidecar process.
    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            eprintln!("stopping mlx_lm.server");
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
    }
}

impl Drop for InferenceServer {
    fn drop(&mut self) {
        self.stop();
    }
}
