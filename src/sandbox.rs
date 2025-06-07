use mustache::MapBuilder;
use std::collections::HashSet;

pub struct SecureLLMSandbox {
    allowed_keys: HashSet<String>,
    template: mustache::Template,
}

impl SecureLLMSandbox {
    pub fn new(template_str: &str, allowed: &[&str]) -> anyhow::Result<Self> {
        let template = mustache::compile_str(template_str)?;
        Ok(Self {
            allowed_keys: allowed.iter().map(|s| s.to_string()).collect(),
            template,
        })
    }

    pub fn render(
        &self,
        data: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<String> {
        for k in data.keys() {
            if !self.allowed_keys.contains(k) {
                return Err(anyhow::anyhow!("key not allowed"));
            }
        }
        let mut builder = MapBuilder::new();
        for (k, v) in data {
            builder = builder.insert_str(k, v);
        }
        let data = builder.build();
        let mut bytes = Vec::new();
        self.template.render_data(&mut bytes, &data)?;
        Ok(String::from_utf8(bytes).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_render() {
        let sandbox = SecureLLMSandbox::new("Hello {{name}}", &["name"]).unwrap();
        let mut map = std::collections::HashMap::new();
        map.insert("name".into(), "World".into());
        let out = sandbox.render(&map).unwrap();
        assert_eq!(out.trim(), "Hello World");
    }

    #[test]
    fn disallow_key() {
        let sandbox = SecureLLMSandbox::new("Hi", &[]).unwrap();
        let mut map = std::collections::HashMap::new();
        map.insert("bad".into(), "1".into());
        assert!(sandbox.render(&map).is_err());
    }
}
