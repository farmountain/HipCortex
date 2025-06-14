use hipcortex::llm_clients::llama::LlamaClient;
use hipcortex::llm_clients::LLMClient;
use mockito::Server;

#[test]
fn llama_client_sends_request() {
    let mut server = Server::new();
    let m = server
        .mock("POST", "/v1/generate")
        .with_status(200)
        .with_body("{\"response\":\"ok\"}")
        .create();
    let client = LlamaClient::new(server.url());
    let resp = client.generate_response("hi");
    m.assert();
    assert_eq!(resp, "ok");
}
