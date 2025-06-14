use hipcortex::knowledge_export::{NotionExporter, PdfExporter};
use mockito::{Matcher, Server};

#[test]
fn pdf_export_creates_file() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    PdfExporter::export_to_file(tmp.path().to_str().unwrap(), "hello").unwrap();
    assert!(tmp.path().exists());
}

#[test]
fn notion_export_sends_request() {
    let mut server = Server::new();
    let m = server
        .mock("POST", "/pages")
        .match_body(Matcher::Any)
        .with_status(200)
        .create();
    let exporter = NotionExporter::with_base_url("token", &server.url());
    exporter.export_page("t", "c").unwrap();
    m.assert();
}
