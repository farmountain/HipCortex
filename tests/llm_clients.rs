use hipcortex::llm_clients::deepseek_client::DeepSeekClient;
use hipcortex::llm_clients::falcon_client::FalconClient;
use hipcortex::llm_clients::local_llm_client::LocalLLMClient;
use hipcortex::llm_clients::mistral_client::MistralClient;
use hipcortex::llm_clients::LanguageModelClient;
use mockito::Server;

#[test]
fn mistral_client_endpoints() {
    let mut server = Server::new();
    let m1 = server
        .mock("POST", "/generate")
        .with_status(200)
        .with_body("{\"text\":\"gen\"}")
        .create();
    let m2 = server
        .mock("POST", "/embed")
        .with_status(200)
        .with_body("{\"embedding\":[1,2]}")
        .create();
    let m3 = server
        .mock("POST", "/chat")
        .with_status(200)
        .with_body("{\"reply\":\"hi\"}")
        .create();
    let client = MistralClient::new(server.url());
    assert_eq!(client.generate("p"), "gen");
    assert_eq!(client.embed("t"), vec![1.0, 2.0]);
    assert_eq!(client.chat(vec!["a".into()], "b"), "hi");
    m1.assert();
    m2.assert();
    m3.assert();
}

#[test]
fn falcon_client_endpoints() {
    let mut server = Server::new();
    server
        .mock("POST", "/generate")
        .with_status(200)
        .with_body("{\"text\":\"ok\"}")
        .create();
    server
        .mock("POST", "/embed")
        .with_status(200)
        .with_body("{\"embedding\":[3]}")
        .create();
    server
        .mock("POST", "/chat")
        .with_status(200)
        .with_body("{\"reply\":\"c\"}")
        .create();
    let client = FalconClient::new(server.url());
    assert_eq!(client.generate("x"), "ok");
    assert_eq!(client.embed("t"), vec![3.0]);
    assert_eq!(client.chat(vec![], "n"), "c");
}

#[test]
fn deepseek_client_endpoints() {
    let mut server = Server::new();
    server
        .mock("POST", "/generate")
        .with_status(200)
        .with_body("{\"text\":\"d\"}")
        .create();
    server
        .mock("POST", "/embed")
        .with_status(200)
        .with_body("{\"embedding\":[4,5]}")
        .create();
    server
        .mock("POST", "/chat")
        .with_status(200)
        .with_body("{\"reply\":\"r\"}")
        .create();
    let client = DeepSeekClient::new(server.url());
    assert_eq!(client.generate("a"), "d");
    assert_eq!(client.embed("b"), vec![4.0, 5.0]);
    assert_eq!(client.chat(vec!["1".into()], "2"), "r");
}

#[test]
fn local_llm_client_runs_command() {
    let client = LocalLLMClient::new("echo");
    assert_eq!(client.generate("hello"), "hello");
    assert_eq!(client.chat(vec!["a".into()], "b"), "a\nb");
    assert!(client.embed("x").is_empty());
}
