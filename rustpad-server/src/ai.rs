//! AI integration with OpenRouter API for document assistance.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use log::info;

/// Configuration for AI features
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Whether AI features are enabled
    pub enabled: bool,
    /// OpenRouter API key
    pub api_key: String,
    /// Optional custom base URL for OpenRouter
    pub base_url: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }
}

impl AiConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let enabled = std::env::var("ENABLE_AI")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);
        
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .unwrap_or_default();
        
        let base_url = std::env::var("OPENROUTER_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

        if enabled && api_key.is_empty() {
            log::warn!("AI features enabled but OPENROUTER_API_KEY not set");
        }

        Self {
            enabled,
            api_key,
            base_url,
        }
    }
}

/// Message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender (user, assistant, system)
    pub role: String,
    /// Content of the message
    pub content: String,
}

/// Request to OpenRouter chat completion API
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

/// Response from OpenRouter chat completion API
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Unique identifier for the completion
    pub id: String,
    /// List of completion choices
    pub choices: Vec<ChatChoice>,
    /// Token usage information
    pub usage: Option<Usage>,
}

/// A single completion choice from the API
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The generated message
    pub message: ChatMessage,
    /// Reason why the completion finished
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Available AI model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier
    pub id: String,
    /// Human-readable model name
    pub name: String,
    /// Model description
    pub description: String,
    /// Maximum context window size
    pub context_length: u32,
    /// Pricing information
    pub pricing: ModelPricing,
}

/// Pricing information for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per million prompt tokens
    pub prompt: String,
    /// Cost per million completion tokens
    pub completion: String,
}

/// Manager for AI operations
#[derive(Debug)]
pub struct AiManager {
    config: AiConfig,
    client: reqwest::Client,
}

impl AiManager {
    /// Create a new AI manager
    pub fn new(config: AiConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        if config.enabled {
            info!("AI features enabled with OpenRouter");
        }

        Ok(Self { config, client })
    }

    /// Check if AI is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.api_key.is_empty()
    }

    /// Get available models (curated list for document editing)
    pub fn get_available_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "anthropic/claude-3.5-sonnet".to_string(),
                name: "Claude 3.5 Sonnet".to_string(),
                description: "Most capable model, excellent for code and writing".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    prompt: "$3/M tokens".to_string(),
                    completion: "$15/M tokens".to_string(),
                },
            },
            ModelInfo {
                id: "anthropic/claude-3-haiku".to_string(),
                name: "Claude 3 Haiku".to_string(),
                description: "Fast and efficient for simple tasks".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    prompt: "$0.25/M tokens".to_string(),
                    completion: "$1.25/M tokens".to_string(),
                },
            },
            ModelInfo {
                id: "openai/gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                description: "Powerful OpenAI model with good reasoning".to_string(),
                context_length: 128000,
                pricing: ModelPricing {
                    prompt: "$10/M tokens".to_string(),
                    completion: "$30/M tokens".to_string(),
                },
            },
            ModelInfo {
                id: "openai/gpt-3.5-turbo".to_string(),
                name: "GPT-3.5 Turbo".to_string(),
                description: "Fast and cost-effective".to_string(),
                context_length: 16385,
                pricing: ModelPricing {
                    prompt: "$0.50/M tokens".to_string(),
                    completion: "$1.50/M tokens".to_string(),
                },
            },
            ModelInfo {
                id: "google/gemini-pro-1.5".to_string(),
                name: "Gemini Pro 1.5".to_string(),
                description: "Google's powerful model with large context".to_string(),
                context_length: 1000000,
                pricing: ModelPricing {
                    prompt: "$2.50/M tokens".to_string(),
                    completion: "$10/M tokens".to_string(),
                },
            },
        ]
    }

    /// Send a chat completion request
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<ChatCompletionResponse> {
        if !self.is_enabled() {
            anyhow::bail!("AI features are not enabled");
        }

        let url = format!("{}/chat/completions", self.config.base_url);
        
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            max_tokens,
            temperature,
            stream: false,
        };

        info!("Sending chat completion request to OpenRouter with model: {}", model);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("HTTP-Referer", "https://rustpad.io")
            .header("X-Title", "Rustpad")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("OpenRouter API error ({}): {}", status, error_text);
        }

        let completion = response
            .json::<ChatCompletionResponse>()
            .await
            .context("Failed to parse OpenRouter response")?;

        info!(
            "Chat completion successful, tokens used: {:?}",
            completion.usage
        );

        Ok(completion)
    }
}
