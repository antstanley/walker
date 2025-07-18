use walker::{
    error::Result,
    parsers::exports::ExportsParser,
};
use serde_json::json;

#[test]
fn test_exports_has_condition() {
    let exports = json!({
        ".": {
            "import": "./index.mjs",
            "require": "./index.cjs"
        },
        "./utils": "./utils.js"
    });

    assert!(ExportsParser::exports_has_condition(&exports, "import"));
    assert!(ExportsParser::exports_has_condition(&exports, "require"));
    assert!(!ExportsParser::exports_has_condition(&exports, "types"));
    assert!(!ExportsParser::exports_has_condition(&exports, "browser"));
}

#[test]
fn test_nested_exports_has_condition() {
    let exports = json!({
        ".": {
            "node": {
                "import": "./node.mjs",
                "require": "./node.cjs"
            },
            "browser": {
                "import": "./browser.mjs"
            }
        }
    });

    assert!(ExportsParser::exports_has_condition(&exports, "node"));
    assert!(ExportsParser::exports_has_condition(&exports, "browser"));
    assert!(ExportsParser::exports_has_condition(&exports, "import"));
    assert!(ExportsParser::exports_has_condition(&exports, "require"));
}

#[test]
fn test_collect_export_extensions() {
    let exports = json!({
        ".": {
            "import": "./index.mjs",
            "require": "./index.cjs",
            "types": "./index.d.ts"
        },
        "./utils": "./utils.js"
    });

    let extensions = ExportsParser::collect_export_extensions(&exports);
    assert!(extensions.contains(".mjs"));
    assert!(extensions.contains(".cjs"));
    assert!(extensions.contains(".d.ts"));
    assert!(extensions.contains(".js"));
}

#[test]
fn test_extract_extension() {
    assert_eq!(ExportsParser::extract_extension("./index.mjs"), Some(".mjs".to_string()));
    assert_eq!(ExportsParser::extract_extension("./dist/index.cjs"), Some(".cjs".to_string()));
    assert_eq!(ExportsParser::extract_extension("./types/index.d.ts"), Some(".d.ts".to_string()));
    assert_eq!(ExportsParser::extract_extension("./file"), None);
    assert_eq!(ExportsParser::extract_extension("https://example.com/file.js"), None);
    assert_eq!(ExportsParser::extract_extension("."), None);
}

#[test]
fn test_analyze_exports_simple() {
    let exports = json!({
        ".": "./index.js"
    });

    let (has_esm, has_cjs, has_typescript, has_browser) = ExportsParser::analyze_exports(&exports);
    assert!(has_esm);  // Default assumption for simple exports
    assert!(has_cjs);  // Default assumption for simple exports
    assert!(!has_typescript);
    assert!(!has_browser);
}

#[test]
fn test_analyze_exports_explicit() {
    let exports = json!({
        ".": {
            "import": "./index.mjs",
            "require": "./index.cjs",
            "types": "./index.d.ts"
        }
    });

    let (has_esm, has_cjs, has_typescript, has_browser) = ExportsParser::analyze_exports(&exports);
    assert!(has_esm);
    assert!(has_cjs);
    assert!(has_typescript);
    assert!(!has_browser);
}

#[test]
fn test_analyze_exports_browser() {
    let exports = json!({
        ".": {
            "browser": {
                "import": "./browser.mjs",
                "require": "./browser.cjs"
            },
            "node": {
                "import": "./node.mjs",
                "require": "./node.cjs"
            }
        }
    });

    let (has_esm, has_cjs, has_typescript, has_browser) = ExportsParser::analyze_exports(&exports);
    assert!(has_esm);
    assert!(has_cjs);
    assert!(!has_typescript);
    assert!(has_browser);
}

#[test]
fn test_create_module_support() {
    let exports = json!({
        ".": {
            "import": "./index.mjs",
            "require": "./index.cjs",
            "types": "./index.d.ts"
        }
    });

    // Test with type: module
    let module_support = ExportsParser::create_module_support(&exports, Some("module"));
    assert!(module_support.esm.type_module);
    assert!(module_support.esm.exports_import);
    assert!(module_support.esm.overall);
    assert!(module_support.cjs.exports_require);
    assert!(module_support.cjs.overall);
    assert!(module_support.typescript.exports_dts);
    assert!(module_support.typescript.overall);

    // Test with type: commonjs
    let module_support = ExportsParser::create_module_support(&exports, Some("commonjs"));
    assert!(!module_support.esm.type_module);
    assert!(module_support.esm.exports_import);
    assert!(module_support.esm.overall);
    assert!(module_support.cjs.type_commonjs);
    assert!(module_support.cjs.exports_require);
    assert!(module_support.cjs.overall);
}

#[test]
fn test_complex_conditional_exports() {
    let exports = json!({
        ".": {
            "types": "./dist/index.d.ts",
            "import": {
                "node": "./dist/node/index.mjs",
                "default": "./dist/index.mjs"
            },
            "require": {
                "node": "./dist/node/index.cjs",
                "default": "./dist/index.js"
            },
            "browser": {
                "import": "./dist/browser/index.mjs",
                "require": "./dist/browser/index.js"
            }
        }
    });

    let (has_esm, has_cjs, has_typescript, has_browser) = ExportsParser::analyze_exports(&exports);
    assert!(has_esm);
    assert!(has_cjs);
    assert!(has_typescript);
    assert!(has_browser);

    let module_support = ExportsParser::create_module_support(&exports, None);
    assert!(module_support.esm.exports_import);
    assert!(module_support.esm.overall);
    assert!(module_support.cjs.exports_require);
    assert!(module_support.cjs.overall);
    assert!(module_support.typescript.exports_dts);
    assert!(module_support.typescript.overall);
    assert!(module_support.browser.exports_browser);
    assert!(module_support.browser.overall);
}