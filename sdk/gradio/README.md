# HipCortex Gradio Demo

Interactive web UI for HipCortex — add memories, search, export, GDPR forget.

## Deploy to HuggingFace Spaces (free)

1. Go to https://huggingface.co/new-space
2. Name: `hipcortex-demo`
3. SDK: **Gradio**
4. Create Space, then upload `app.py` and `requirements.txt`
5. Add secrets (Space Settings → Variables and secrets):
   - `HIPCORTEX_URL` = `https://hipcortex.fly.dev` (or your instance)
   - `HIPCORTEX_API_KEY` = your API key (optional, for free tier leave blank)
6. Space starts automatically — share the URL

## Run locally

```bash
pip install gradio requests
HIPCORTEX_URL=http://localhost:3030 python app.py
```
Open http://localhost:7860

## Features

- **Add Memory** — store memories with optional TTL auto-expiry
- **Search** — keyword search with actor filter
- **Stats & Export** — view stats, download all records as text
- **GDPR Forget** — delete all memories for an actor
