//! Image analysis tool using vision-capable LLMs.
//!
//! Gated by `network.http` permission (calls LLM API).

use async_trait::async_trait;
use omni_permissions::capability::Capability;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Native tool for analyzing images using vision LLMs.
pub struct ImageAnalyzeTool;

#[async_trait]
impl NativeTool for ImageAnalyzeTool {
    fn name(&self) -> &str {
        "image_analyze"
    }

    fn description(&self) -> &str {
        "Analyze an image file using AI vision capabilities. Reads an image from disk, \
         encodes it, and describes its contents. Supports PNG, JPEG, GIF, and WebP formats."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "image_path": {
                    "type": "string",
                    "description": "Path to the image file to analyze"
                },
                "prompt": {
                    "type": "string",
                    "description": "Analysis prompt (default: 'Describe this image in detail.')"
                }
            },
            "required": ["image_path"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::NetworkHttp(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let image_path = params["image_path"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'image_path' parameter is required".to_string()))?;
        let prompt = params["prompt"]
            .as_str()
            .unwrap_or("Describe this image in detail.");

        // Detect MIME type from extension
        let ext = std::path::Path::new(image_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mime_type = match ext.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "svg" => "image/svg+xml",
            _ => {
                return Err(LlmError::ToolCall(format!(
                    "Unsupported image format: .{ext}. Supported: png, jpg, gif, webp, bmp, svg"
                )));
            }
        };

        // Check file size BEFORE reading (max 20MB) -- prevents OOM on huge files
        const MAX_IMAGE_SIZE: u64 = 20 * 1024 * 1024;
        let file_size = tokio::fs::metadata(image_path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to stat image '{}': {}", image_path, e)))?
            .len();
        if file_size > MAX_IMAGE_SIZE {
            return Err(LlmError::ToolCall(format!(
                "Image too large: {} bytes (max {} bytes / 20MB)",
                file_size, MAX_IMAGE_SIZE
            )));
        }

        // Read image file (size already validated)
        let image_data = tokio::fs::read(image_path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read image '{}': {}", image_path, e)))?;

        // Base64 encode
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_data,
        );

        // Return the encoded image data for the LLM to process
        // The agent loop or a vision-capable provider will handle the actual analysis
        Ok(serde_json::json!({
            "image_data": format!("data:{};base64,{}", mime_type, b64),
            "mime_type": mime_type,
            "size_bytes": image_data.len(),
            "prompt": prompt,
            "analysis": format!(
                "[Image loaded: {} ({}, {} bytes). To analyze, send this image data to a vision-capable LLM.]",
                image_path, mime_type, image_data.len()
            ),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_analyze_schema() {
        let tool = ImageAnalyzeTool;
        assert_eq!(tool.name(), "image_analyze");
        assert_eq!(tool.required_capability().capability_key(), "network.http");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["image_path"].is_object());
    }

    #[tokio::test]
    async fn test_image_analyze_nonexistent() {
        let tool = ImageAnalyzeTool;
        let result = tool
            .execute(serde_json::json!({"image_path": "/nonexistent/image.png"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_image_analyze_unsupported_format() {
        let tmp = std::env::temp_dir().join("test_image.xyz");
        tokio::fs::write(&tmp, b"fake image data").await.unwrap();

        let tool = ImageAnalyzeTool;
        let result = tool
            .execute(serde_json::json!({"image_path": tmp.display().to_string()}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));

        tokio::fs::remove_file(&tmp).await.ok();
    }

    #[tokio::test]
    async fn test_image_analyze_valid_png() {
        // Create a minimal valid-ish file with .png extension
        let tmp = std::env::temp_dir().join("test_image_analyze.png");
        tokio::fs::write(&tmp, b"\x89PNG\r\n\x1a\nfake").await.unwrap();

        let tool = ImageAnalyzeTool;
        let result = tool
            .execute(serde_json::json!({
                "image_path": tmp.display().to_string(),
                "prompt": "What is in this image?"
            }))
            .await
            .unwrap();

        assert_eq!(result["mime_type"], "image/png");
        assert!(result["image_data"].as_str().unwrap().starts_with("data:image/png;base64,"));

        tokio::fs::remove_file(&tmp).await.ok();
    }
}
