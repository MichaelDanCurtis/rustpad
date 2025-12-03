# AI Model Selection

## Overview
The AI integration now supports **all models available on OpenRouter**, including the auto-router that automatically selects the best model for your request.

## Changes Made

### Dynamic Model Fetching
- Added `get_available_models_async()` method that fetches all models from OpenRouter API in real-time
- The `/api/ai/models` endpoint now returns the complete list of available models
- Includes the **"auto" model** which automatically routes to the best model

### Fallback Support
If the OpenRouter API is unavailable, the system falls back to a curated list of 6 models:
1. **Auto (Best)** - Auto-router (NEW!)
2. Claude 3.5 Sonnet
3. Claude 3 Haiku
4. GPT-4 Turbo
5. GPT-3.5 Turbo
6. Gemini Pro 1.5

## How It Works

### API Structure
```rust
// OpenRouter returns models in this format
{
  "data": [
    {
      "id": "anthropic/claude-3.5-sonnet",
      "name": "Claude 3.5 Sonnet",
      "description": "Most capable model...",
      "context_length": 200000,
      "pricing": {
        "prompt": "0.000003",
        "completion": "0.000015"
      }
    },
    ...
  ]
}
```

### Frontend Integration
When your frontend calls `GET /api/ai/models`, it will receive:
- All available models from OpenRouter
- Model pricing information
- Context window sizes
- Model descriptions

### Using the Auto Router
To use the auto-router, simply select the model with ID `"auto"` when making chat requests:

```javascript
POST /api/ai/chat
{
  "model": "auto",
  "messages": [
    {"role": "user", "content": "Your prompt here"}
  ]
}
```

## Benefits
1. **Always up-to-date**: New models automatically appear as OpenRouter adds them
2. **Auto-routing**: Let OpenRouter choose the best model for your task
3. **Full selection**: Access to all models, not just 5 pre-selected ones
4. **Reliable**: Falls back to curated list if API is unavailable

## Testing
To test the models endpoint:
```bash
# Set your API key
$env:OPENROUTER_API_KEY = "sk-or-v1-..."
$env:ENABLE_AI = "true"

# Start the server
cargo run

# In another terminal, test the endpoint
curl http://localhost:8000/api/ai/models
```

You should see a comprehensive list of all OpenRouter models, with "Auto (Best)" at the top.
