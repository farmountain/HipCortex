use crate::perception_adapter::{Modality, PerceptInput};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct OpenManusRequest {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct OpenManusResponse {
    pub role: String,
    pub content: String,
}

pub fn decode_request(json: &str) -> serde_json::Result<PerceptInput> {
    let req: OpenManusRequest = serde_json::from_str(json)?;
    Ok(PerceptInput {
        modality: Modality::AgentMessage,
        text: Some(req.content),
        embedding: None,
        image_data: None,
        tags: vec![req.role],
    })
}

pub fn encode_response(role: &str, text: &str) -> String {
    let resp = OpenManusResponse {
        role: role.to_string(),
        content: text.to_string(),
    };
    serde_json::to_string(&resp).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let req = OpenManusRequest {
            role: "user".into(),
            content: "ping".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let percept = decode_request(&json).unwrap();
        assert_eq!(percept.text.unwrap(), "ping");
        assert_eq!(percept.tags[0], "user");

        let json_resp = encode_response("assistant", "pong");
        let resp: OpenManusResponse = serde_json::from_str(&json_resp).unwrap();
        assert_eq!(resp.role, "assistant");
        assert_eq!(resp.content, "pong");
    }
}
