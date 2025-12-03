//! AI integration with OpenRouter API for document assistance.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use log::info;
use std::sync::{Arc, RwLock};

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

/// OpenRouter API model response
#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    context_length: u32,
    pricing: OpenRouterPricing,
}

/// OpenRouter API pricing structure
#[derive(Debug, Deserialize)]
struct OpenRouterPricing {
    prompt: String,
    completion: String,
}

/// OpenRouter models list response
#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
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
pub struct AiManager {
    config: Arc<RwLock<AiConfig>>,
    client: reqwest::Client,
}

impl std::fmt::Debug for AiManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiManager")
            .field("config", &self.config)
            .field("client", &"<reqwest::Client>")
            .finish()
    }
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

        Ok(Self { 
            config: Arc::new(RwLock::new(config)),
            client 
        })
    }

    /// Check if AI is enabled
    pub fn is_enabled(&self) -> bool {
        let config = self.config.read().unwrap();
        config.enabled && !config.api_key.is_empty()
    }

    /// Get the current API key
    pub fn get_api_key(&self) -> String {
        let config = self.config.read().unwrap();
        config.api_key.clone()
    }

    /// Update the API key
    pub fn update_api_key(&self, new_key: &str) -> Result<()> {
        let mut config = self.config.write().unwrap();
        let trimmed_key = new_key.trim().to_string();
        config.api_key = trimmed_key.clone();
        info!("OpenRouter API key updated (length: {})", trimmed_key.len());
        Ok(())
    }

    /// Get available models (returns fallback list synchronously)
    /// For full dynamic list, use get_available_models_async()
    pub fn get_available_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "openrouter/auto".to_string(),
                name: "Auto Router (Recommended)".to_string(),
                description: "Automatically routes to the best model for your task".to_string(),
                context_length: 2000000,
                pricing: ModelPricing {
                    prompt: "Varies by model".to_string(),
                    completion: "Varies by model".to_string(),
                },
            },
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

    /// Fetch all available models from OpenRouter API
    pub async fn get_available_models_async(&self) -> Result<Vec<ModelInfo>> {
        let (url, api_key) = {
            let config = self.config.read().unwrap();
            (format!("{}/models", config.base_url), config.api_key.clone())
        };

        info!("Fetching available models from OpenRouter API");

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .context("Failed to fetch models from OpenRouter")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            log::warn!("Failed to fetch models from OpenRouter ({}): {}", status, error_text);
            // Return fallback list if API call fails
            return Ok(self.get_available_models());
        }

        let models_response = response
            .json::<OpenRouterModelsResponse>()
            .await
            .context("Failed to parse OpenRouter models response")?;

        let mut models: Vec<ModelInfo> = models_response.data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id,
                name: m.name,
                description: m.description.unwrap_or_else(|| "No description available".to_string()),
                context_length: m.context_length,
                pricing: ModelPricing {
                    prompt: m.pricing.prompt,
                    completion: m.pricing.completion,
                },
            })
            .collect();

        // Add auto router at the beginning if not already present
        if !models.iter().any(|m| m.id == "auto") {
            models.insert(0, ModelInfo {
                id: "auto".to_string(),
                name: "Auto (Best)".to_string(),
                description: "Automatically selects the best model for your request".to_string(),
                context_length: 200000,
                pricing: ModelPricing {
                    prompt: "Variable".to_string(),
                    completion: "Variable".to_string(),
                },
            });
        }

        info!("Successfully fetched {} models from OpenRouter", models.len());
        Ok(models)
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

        let (url, api_key) = {
            let config = self.config.read().unwrap();
            (format!("{}/chat/completions", config.base_url), config.api_key.clone())
        };
        
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            max_tokens,
            temperature,
            stream: false,
        };

        info!("Sending chat completion request to OpenRouter with model: {}", model);
        info!("API key length: {}, starts with: {}", api_key.len(), &api_key[..15.min(api_key.len())]);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
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
