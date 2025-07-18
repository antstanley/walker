use std::path::Path;
use walker::{
    error::Result,
    models::package::{AuthorInfo, ModuleSupport, PackageDetails, RepositoryInfo},
    parsers::package_json::PackageJsonParser,
};

#[test]
fn test_parse_esm_only_package() -> Result<()> {
    let path = Path::new("tests/fixtures/packages/esm-only/package.json");
    let details = PackageJsonParser::parse_file(path)?;

    assert_eq!(details.name, "esm-only-package");
    assert_eq!(details.version, "1.0.0");
    assert_eq!(details.description, Some("ESM-only package for testing".to_string()));
    assert_eq!(details.package_type, Some("module".to_string()));
    assert_eq!(details.main, Some("index.js".to_string()));
    assert!(details.exports.is_some());
    assert_eq!(details.license, Some("MIT".to_string()));

    // Test module support detection
    let module_support = ModuleSupport::from_package_details(&details);
    assert!(module_support.esm.type_module);
    assert!(module_support.esm.overall);
    assert!(!module_support.cjs.overall);
    assert!(module_support.is_esm_only());
    assert!(!module_support.is_dual_mode());

    Ok(())
}

#[test]
fn test_parse_cjs_only_package() -> Result<()> {
    let path = Path::new("tests/fixtures/packages/cjs-only/package.json");
    let details = PackageJsonParser::parse_file(path)?;

    assert_eq!(details.name, "cjs-only-package");
    assert_eq!(details.version, "1.0.0");
    assert_eq!(details.description, Some("CommonJS-only package for testing".to_string()));
    assert_eq!(details.package_type, Some("commonjs".to_string()));
    assert_eq!(details.main, Some("index.js".to_string()));
    assert_eq!(details.license, Some("MIT".to_string()));
    assert!(details.dependencies.is_some());
    assert!(details.dev_dependencies.is_some());

    // Test module support detection
    let module_support = ModuleSupport::from_package_details(&details);
    assert!(module_support.cjs.type_commonjs);
    assert!(module_support.cjs.overall);
    assert!(!module_support.esm.overall);
    assert!(module_support.is_cjs_only());
    assert!(!module_support.is_dual_mode());

    // Test dependency extraction
    let dep_info = walker::parsers::package_json::PackageJsonParser::extract_dependency_info(&details);
    assert_eq!(dep_info.production_count, 1);
    assert_eq!(dep_info.development_count, 1);
    assert_eq!(dep_info.total_count, 2);

    Ok(())
}

#[test]
fn test_parse_dual_mode_package() -> Result<()> {
    let path = Path::new("tests/fixtures/packages/dual-mode/package.json");
    let details = PackageJsonParser::parse_file(path)?;

    assert_eq!(details.name, "dual-mode-package");
    assert_eq!(details.version, "1.0.0");
    assert_eq!(details.main, Some("index.cjs".to_string()));
    assert_eq!(details.module, Some("index.mjs".to_string()));
    assert_eq!(details.types, Some("index.d.ts".to_string()));
    assert!(details.exports.is_some());

    // Test author parsing
    match &details.author {
        Some(AuthorInfo::Object { name, email, url }) => {
            assert_eq!(name, "Test Author");
            assert_eq!(email, &Some("test@example.com".to_string()));
            assert_eq!(url, &Some("https://example.com".to_string()));
        }
        _ => panic!("Expected author object"),
    }

    // Test repository parsing
    match &details.repository {
        Some(RepositoryInfo::Object { repo_type, url, directory }) => {
            assert_eq!(repo_type, "git");
            assert_eq!(url, "https://github.com/example/dual-mode-package");
            assert_eq!(directory, &None);
        }
        _ => panic!("Expected repository object"),
    }

    // Test module support detection
    let module_support = ModuleSupport::from_package_details(&details);
    assert!(module_support.esm.module_field);
    assert!(module_support.esm.exports_import);
    assert!(module_support.esm.overall);
    assert!(module_support.cjs.exports_require);
    assert!(module_support.cjs.overall);
    assert!(module_support.typescript.types_field);
    assert!(module_support.typescript.overall);
    assert!(module_support.is_dual_mode());
    assert!(!module_support.is_esm_only());
    assert!(!module_support.is_cjs_only());

    Ok(())
}

#[test]
fn test_parse_complex_exports_package() -> Result<()> {
    let path = Path::new("tests/fixtures/packages/complex-exports/package.json");
    let details = PackageJsonParser::parse_file(path)?;

    assert_eq!(details.name, "complex-exports-package");
    assert_eq!(details.version, "1.0.0");
    assert_eq!(details.main, Some("./dist/index.js".to_string()));
    assert_eq!(details.module, Some("./dist/index.mjs".to_string()));
    assert_eq!(details.types, Some("./dist/index.d.ts".to_string()));
    assert!(details.exports.is_some());
    assert!(details.browser.is_some());
    assert!(details.engines.is_some());

    // Test node version requirement
    assert_eq!(details.node_version_requirement(), Some(">=14.0.0".to_string()));

    // Test module support detection
    let module_support = ModuleSupport::from_package_details(&details);
    assert!(module_support.esm.module_field);
    assert!(module_support.esm.exports_import);
    assert!(module_support.esm.overall);
    assert!(module_support.cjs.exports_require);
    assert!(module_support.cjs.overall);
    assert!(module_support.typescript.types_field);
    assert!(module_support.typescript.overall);
    assert!(module_support.browser.browser_field);
    assert!(module_support.browser.exports_browser);
    assert!(module_support.browser.overall);
    assert!(module_support.is_dual_mode());
    assert!(module_support.has_typescript_support());
    assert!(module_support.has_browser_support());

    // Test dependency extraction
    let dep_info = walker::parsers::package_json::PackageJsonParser::extract_dependency_info(&details);
    assert_eq!(dep_info.production_count, 1);
    assert_eq!(dep_info.peer_count, 1);
    assert_eq!(dep_info.optional_count, 1);
    assert_eq!(dep_info.total_count, 3);

    Ok(())
}

#[test]
fn test_parse_invalid_json() {
    let invalid_json = r#"{ "name": "invalid-json", "version": "1.0.0", invalid }"#;
    let result = PackageJsonParser::parse(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_parse_missing_required_fields() {
    // Missing name
    let missing_name = r#"{ "version": "1.0.0" }"#;
    let result = PackageJsonParser::parse(missing_name);
    assert!(result.is_err());

    // Missing version
    let missing_version = r#"{ "name": "test-package" }"#;
    let result = PackageJsonParser::parse(missing_version);
    assert!(result.is_err());
}

#[test]
fn test_extract_dependency_info() -> Result<()> {
    let json = r#"{
        "name": "test-package",
        "version": "1.0.0",
        "dependencies": {
            "lodash": "^4.17.21",
            "express": "^4.17.1"
        },
        "devDependencies": {
            "jest": "^27.0.0"
        },
        "peerDependencies": {
            "react": "^17.0.0"
        },
        "optionalDependencies": {
            "fsevents": "^2.3.2"
        }
    }"#;

    let details = PackageJsonParser::parse(json)?;
    let dep_info = PackageJsonParser::extract_dependency_info(&details);

    assert_eq!(dep_info.production_count, 2);
    assert_eq!(dep_info.development_count, 1);
    assert_eq!(dep_info.peer_count, 1);
    assert_eq!(dep_info.optional_count, 1);
    assert_eq!(dep_info.total_count, 5);

    // Check dependency names
    let all_deps = dep_info.all_dependency_names();
    assert!(all_deps.contains(&"lodash".to_string()));
    assert!(all_deps.contains(&"express".to_string()));
    assert!(all_deps.contains(&"jest".to_string()));
    assert!(all_deps.contains(&"react".to_string()));
    assert!(all_deps.contains(&"fsevents".to_string()));

    Ok(())
}