use std::collections::HashMap;
use tokr_driver::compile;
use tokr_ir::PassConfig;

#[test]
fn test_golden_sample() {
    let src = include_str!("../../../tests/fixtures/sample.scss");
    let mut order_table = HashMap::new();
    order_table.insert(
        "colors".to_string(),
        vec!["primary".to_string(), "secondary".to_string()],
    );

    let cfg = PassConfig {
        strict: false,
        order_table,
    };

    let (ts, _dts) = compile(src, &cfg, None, false, false).expect("Should compile successfully");

    // Assert against some expected structures in the TS output
    assert!(ts.contains("export const theme ="));
    assert!(ts.contains("colors: {"));
    assert!(ts.contains("primary: \"#ff0000\""));
    assert!(ts.contains("button: {"));
}

#[test]
fn test_box_shadow_array() {
    let src = "
    /* @theme boxShadow[0] */
    $shadow-sm: 0 1px 2px rgba(0,0,0,0.1);
    
    /* @theme boxShadow[2] */
    $shadow-lg: 0 4px 6px rgba(0,0,0,0.1);
    ";

    let cfg = PassConfig {
        strict: false,
        order_table: HashMap::new(),
    };

    let (ts, _dts) = compile(src, &cfg, None, false, false).expect("Should compile successfully");

    // Expected hole filling at index 1
    assert!(ts.contains("boxShadow: ["));
    assert!(ts.contains("\"0 1px 2px rgba(0,0,0,0.1)\""));
    assert!(ts.contains("undefined"));
    assert!(ts.contains("\"0 4px 6px rgba(0,0,0,0.1)\""));
}

#[test]
fn test_css_variables() {
    let src = "
    /* @theme color.accent.base */
    --accent: #fafafa;
    
    /* @theme color.black */
    --black: #000;
    
    /* @theme color.accent.foreground */
    --accent-foreground: var(--black);
    ";

    let cfg = PassConfig {
        strict: false,
        order_table: HashMap::new(),
    };

    let (ts, _dts) = compile(src, &cfg, None, false, false).expect("Should compile successfully");

    assert!(ts.contains("color: {"));
    assert!(ts.contains("accent: {"));
    assert!(ts.contains("base: \"#fafafa\""));
    assert!(ts.contains("black: \"#000\""));
    assert!(ts.contains("foreground: \"var(--black)\""));
}
