#[path = "../src/semantic_console.rs"]
mod semantic_console;

use semantic_console::browser_contract_field_names;

#[test]
fn semantic_console_contract_is_vector_free() {
    let fields = browser_contract_field_names();
    assert!(fields.contains(&"domain"));
    assert!(fields.contains(&"query_text"));
    assert!(fields.contains(&"expected_state"));
    assert!(!fields.contains(&"vector"));
    assert!(!fields.contains(&"embedding"));
}
