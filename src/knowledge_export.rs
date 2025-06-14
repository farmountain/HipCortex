use anyhow::Result;
use printpdf::*;
use reqwest::blocking::Client;
use serde_json::json;
use std::fs::File;
use std::io::BufWriter;

pub struct PdfExporter;

impl PdfExporter {
    pub fn export_to_file(path: &str, text: &str) -> Result<()> {
        let (doc, page1, layer1) = PdfDocument::new("export", Mm(210.0), Mm(297.0), "Layer 1");
        let current_layer = doc.get_page(page1).get_layer(layer1);
        let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        current_layer.use_text(text, 12.0, Mm(10.0), Mm(287.0), &font);
        doc.save(&mut BufWriter::new(File::create(path)?))?;
        Ok(())
    }
}

pub struct NotionExporter {
    token: String,
    client: Client,
    base_url: String,
}

impl NotionExporter {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            client: Client::new(),
            base_url: "https://api.notion.com/v1".to_string(),
        }
    }

    pub fn with_base_url(token: &str, base: &str) -> Self {
        Self {
            token: token.to_string(),
            client: Client::new(),
            base_url: base.to_string(),
        }
    }

    pub fn export_page(&self, title: &str, content: &str) -> Result<()> {
        let body = json!({
            "parent": { "type": "page_id", "page_id": "" },
            "properties": { "title": [{"text": {"content": title}}] },
            "children": [{ "object": "block", "type": "paragraph", "paragraph": {"text": [{"type": "text", "text": {"content": content}}] } }]
        });
        let url = format!("{}/pages", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .header("Notion-Version", "2022-06-28")
            .json(&body)
            .send()?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("request failed: {}", resp.status()))
        }
    }
}
