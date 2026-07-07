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

    let Some((ts, _dts)) =
        compile(src, &cfg, false, None, false, false).expect("Should compile successfully")
    else {
        panic!("expected Some output")
    };

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

    let Some((ts, _dts)) =
        compile(src, &cfg, false, None, false, false).expect("Should compile successfully")
    else {
        panic!("expected Some output")
    };

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

    let Some((ts, _dts)) =
        compile(src, &cfg, false, None, false, false).expect("Should compile successfully")
    else {
        panic!("expected Some output")
    };

    assert!(ts.contains("color: {"));
    assert!(ts.contains("accent: {"));
    assert!(ts.contains("base: \"#fafafa\""));
    assert!(ts.contains("black: \"#000\""));
    assert!(ts.contains("foreground: \"var(--black)\""));
}

#[test]
fn test_css_variable_name_mismatches() {
    let src = "
    /* @theme sidebar.primary */
    --color-sidebar-primary: var(--sidebar-primary);
    
    /* @theme sidebar.primary_foreground */
    --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
    
    /* @theme sidebar.accent */
    --color-sidebar-accent: var(--sidebar-accent);
    
    /* @theme sidebar.accent_foreground */
    --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
    
    /* @theme sidebar.border */
    --color-sidebar-border: var(--sidebar-border);
    
    /* @theme sidebar.ring */
    --color-sidebar-ring: var(--sidebar-ring);
    
    /* @theme radius.lg */
    --radius-lg: var(--radius);
    ";

    let cfg = PassConfig {
        strict: true, // promote warnings to errors
        order_table: HashMap::new(),
    };

    let res = compile(src, &cfg, false, None, false, false);
    assert!(res.is_err());
    let errs = res.unwrap_err().into_vec();

    // Check that we got all 7 expected TC0202 name mismatch errors
    let tc0202_errors: Vec<_> = errs.iter().filter(|d| d.code == "TC0202").collect();
    assert_eq!(tc0202_errors.len(), 7);

    // Verify compile succeeds when allow_name_mismatch is true
    let res_allowed = compile(src, &cfg, true, None, false, false);
    assert!(res_allowed.is_ok());
}
