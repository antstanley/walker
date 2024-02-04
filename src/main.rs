use serde_json::{Map, Value};
use std::env;
use std::fs::{self, DirEntry};
use std::io::{self};
use std::path::PathBuf;

struct ModuleSupport {
    name: String,
    esm_support: bool,
    esm_partial: bool,
    cjs_support: bool,
}

struct PackageValidation {
    is_package: bool,
    module_support: ModuleSupport,
}

// one possible implementation of walking a directory only visiting files
fn walk_dirs(dir: &PathBuf, cb: &dyn Fn(&DirEntry) -> PackageValidation) -> io::Result<()> {
    if dir.is_dir() {
        let mut package_validation = PackageValidation {
            is_package: false,
            module_support: ModuleSupport {
                name: "".to_string(),
                esm_support: false,
                esm_partial: false,
                cjs_support: false,
            },
        };
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk_dirs(&path, cb)?;
            } else {
                let file_package_validation = cb(&entry);
                if file_package_validation.is_package {
                    package_validation = file_package_validation
                }
            }
        }
        if package_validation.is_package {
            println!("Module support for package {:?} - ESM Full: {:?}, ESM Partial ('module' field): {:?}, CommonJS: {:?}", package_validation.module_support.name, package_validation.module_support.esm_support, package_validation.module_support.esm_partial, package_validation.module_support.cjs_support)
        }
    }
    Ok(())
}

fn parse_exports(exports: &Map<String, Value>) -> ModuleSupport {
    const SUB_PATH_PATTERNS: [&str; 4] = ["import", "require", "default", "node"];
    let mut module_support = ModuleSupport {
        name: "".to_string(),
        esm_support: false,
        esm_partial: false,
        cjs_support: false,
    };

    for (key, value) in exports {
        if value.is_string() {
            let key_string = key.as_str();
            if SUB_PATH_PATTERNS.contains(&key_string) {
                if key_string == "import" {
                    module_support.esm_support = true
                } else if key_string == "require" {
                    module_support.cjs_support = true
                }
            }
        } else if value.is_object() {
            // recurse
            let export_module_support = parse_exports(value.as_object().unwrap());
            if export_module_support.esm_support {
                module_support.esm_support = true
            };
            if export_module_support.cjs_support {
                module_support.cjs_support = true
            };
        }
    }

    return module_support;
}

fn parse_package(v: Value) -> ModuleSupport {
    let mut module_support: ModuleSupport = ModuleSupport {
        name: "".to_string(),
        esm_support: false,
        esm_partial: false,
        cjs_support: false,
    };

    // get the package name
    let package_name = v["name"].as_str();

    if package_name.is_some() {
        module_support.name = package_name.unwrap().to_string();
    }

    // check the 'type' field in package.json
    let module_type = v["type"].as_str();

    if module_type.is_some() {
        if module_type.unwrap() == "module" {
            module_support.esm_support = true;
        } else if module_type.unwrap() == "commonjs" {
            module_support.cjs_support = true
        }
    }

    // check the 'module' field in package.json
    let module_field = v["module"].as_str();

    if module_field.is_some() {
        module_support.esm_partial = true;
    }

    // check the 'exports' field in package.json
    let exports = v["exports"].as_object();
    if exports.is_some() {
        let export_module_support = parse_exports(exports.unwrap());

        if export_module_support.esm_support {
            module_support.esm_support = true
        };
        if export_module_support.cjs_support {
            module_support.cjs_support = true
        };
    } else {
        println!("'exports' field not defined")
    }

    return module_support;
}

fn dir_handler(entry: &DirEntry) -> PackageValidation {
    let path = entry.path();
    let file_name = entry.file_name();
    let mut package_validation = PackageValidation {
        is_package: false,
        module_support: ModuleSupport {
            name: "".to_string(),
            esm_support: false,
            esm_partial: false,
            cjs_support: false,
        },
    };

    if file_name == "package.json" {
        package_validation.is_package = true;
        let contents = fs::read_to_string(path).expect("Unable to read file {path}");

        let v: Value = serde_json::from_str(&contents).expect("Unable to parse JSON");

        package_validation.module_support = parse_package(v);
    }
    return package_validation;
}

fn main() {
    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(_) => panic!(),
    };

    let _ = walk_dirs(&current_path, &dir_handler);
}
