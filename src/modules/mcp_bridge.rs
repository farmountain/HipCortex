use crate::perception_adapter::{Modality, PerceptInput};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct McpRequest {
    pub agent: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct McpResponse {
    pub agent: String,
    pub message: String,
}

pub fn decode_request(json: &str) -> serde_json::Result<PerceptInput> {
    let req: McpRequest = serde_json::from_str(json)?;
    Ok(PerceptInput {
        modality: Modality::AgentMessage,
        text: Some(req.message),
        embedding: None,
        image_data: None,
        tags: vec![req.agent],
    })
}

pub fn encode_response(agent: &str, message: &str) -> String {
    let resp = McpResponse {
        agent: agent.to_string(),
        message: message.to_string(),
    };
    serde_json::to_string(&resp).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let req = McpRequest {
            agent: "a1".into(),
            message: "hello".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let percept = decode_request(&json).unwrap();
        assert_eq!(percept.text.unwrap(), "hello");
        assert_eq!(percept.tags[0], "a1");

        let json_resp = encode_response("a1", "ok");
        let resp: McpResponse = serde_json::from_str(&json_resp).unwrap();
        assert_eq!(resp.agent, "a1");
        assert_eq!(resp.message, "ok");
    }
}
