use agents_macros::tool;

// Test basic string parameter
#[tool("Test tool with string param")]
fn test_string(name: String) -> String {
    format!("Hello, {}!", name)
}

#[tokio::test]
async fn test_string_tool() {
    let tool_instance = TestStringTool::as_tool();
    let schema = tool_instance.schema();
    assert_eq!(schema.name, "test_string");
}
