use super::*;

#[test]
fn flags_project_paths_but_not_urls_or_versions() {
    assert!(!reasons(r"tool D:\\Quant\\repo").is_empty());
    assert!(reasons("https://example.test/a").is_empty());
    assert!(reasons("pip install x==1.2.3.4").is_empty());
}
