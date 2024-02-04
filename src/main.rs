use ansi_term::Colour::{Green, Red};
use serde_json::{Map, Value};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::io::{self};
use std::path::{Path, PathBuf};

// from https://stackoverflow.com/questions/45291832/extracting-a-file-extension-from-a-given-path-in-rust-idiomatically
fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

struct ModuleSupport {
    esm_main_mjs: bool,
    esm_type: bool,
    esm_exports: bool,
    esm_partial: bool,
    cjs_type: bool,
    cjs_exports: bool,
}

struct PackageDetails {
    name: String,
    version: String,
    module_support: ModuleSupport,
}

struct PackageValidation {
    is_package: bool,
    package_details: PackageDetails,
}

fn print_result(package_validation: PackageValidation) {
    let PackageDetails {
        module_support,
        name,
        version,
    } = package_validation.package_details;

    let esm = module_support.esm_type
        || module_support.esm_exports
        || module_support.esm_partial
        || module_support.esm_main_mjs;

    let cjs = module_support.cjs_type
        || (!module_support.esm_type
            && !module_support.esm_exports
            && !module_support.esm_partial
            && !module_support.esm_main_mjs);

    let print_esm = match esm {
        true => Green.paint("true"),
        false => Red.paint("false"),
    };

    let print_cjs = match cjs {
        true => Green.paint("true"),
        false => Red.paint("false"),
    };

    println!(
        "Package: {}@{} - ESM Support: {}, CommonJS: {}",
        Green.paint(name),
        Green.paint(version),
        print_esm,
        print_cjs
    );

    if esm {
        let print_esm_type = match module_support.esm_type {
            true => Green.paint("true"),
            false => Red.paint("false"),
        };

        let print_esm_exports = match module_support.esm_exports {
            true => Green.paint("true"),
            false => Red.paint("false"),
        };

        let print_esm_partial = match module_support.esm_partial {
            true => Green.paint("true"),
            false => Red.paint("false"),
        };

        let print_esm_main = match module_support.esm_main_mjs {
            true => Green.paint("true"),
            false => Red.paint("false"),
        };

        println!(
            "'type' set to 'module': {}\n'exports' field defined with 'import' prop: {}\n'module' field set: {}\n'main' field references an '.mjs' file: {}",
            print_esm_type, print_esm_exports, print_esm_partial, print_esm_main
        );
    }
    // println!(
    //     "CommonJS - 'type' set to 'commonjs': {:?}. 'exports' field defined with 'require' prop: {:?}, by default - no ESM config: {:?}",
    //     module_support
    //         .cjs_type,
    //     module_support.cjs_exports, cjs
    // )
}

// one possible implementation of walking a directory only visiting files
fn walk_dirs(dir: &PathBuf, cb: &dyn Fn(&DirEntry) -> PackageValidation) -> io::Result<()> {
    if dir.is_dir() {
        let mut package_validation = PackageValidation {
            is_package: false,
            package_details: PackageDetails {
                name: "".to_string(),
                version: "".to_string(),
                module_support: ModuleSupport {
                    esm_main_mjs: false,
                    esm_type: false,
                    esm_exports: false,
                    esm_partial: false,
                    cjs_type: false,
                    cjs_exports: false,
                },
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
            print_result(package_validation)
        }
    }
    Ok(())
}

fn parse_exports(exports: &Map<String, Value>) -> ModuleSupport {
    const SUB_PATH_PATTERNS: [&str; 4] = ["import", "require", "default", "node"];
    let mut module_support = ModuleSupport {
        esm_main_mjs: false,
        esm_type: false,
        esm_exports: false,
        esm_partial: false,
        cjs_type: false,
        cjs_exports: false,
    };

    for (key, value) in exports {
        if value.is_string() {
            let key_string = key.as_str();
            if SUB_PATH_PATTERNS.contains(&key_string) {
                if key_string == "import" {
                    module_support.esm_exports = true
                } else if key_string == "require" {
                    module_support.cjs_exports = true
                }
            }
        } else if value.is_object() {
            // recurse
            let export_module_support = parse_exports(value.as_object().unwrap());
            if export_module_support.esm_exports {
                module_support.esm_exports = true
            };
            if export_module_support.cjs_exports {
                module_support.cjs_exports = true
            };
        }
    }

    return module_support;
}

fn parse_package(v: Value) -> PackageDetails {
    let mut package_details = PackageDetails {
        name: "".to_string(),
        version: "".to_string(),
        module_support: ModuleSupport {
            esm_main_mjs: false,
            esm_type: false,
            esm_exports: false,
            esm_partial: false,
            cjs_type: false,
            cjs_exports: false,
        },
    };

    // get the package name
    let package_name = v["name"].as_str();

    if package_name.is_some() {
        package_details.name = package_name.unwrap().to_string();
    }

    // get the package name
    let package_version = v["version"].as_str();

    if package_version.is_some() {
        package_details.version = package_version.unwrap().to_string();
    }

    // get main field value
    let main_field = v["main"].as_str();

    if main_field.is_some() {
        let main_extension = get_extension_from_filename(main_field.unwrap());
        if main_extension.is_some() {
            if main_extension.unwrap() == "mjs" {
                package_details.module_support.esm_main_mjs = true
            }
        }
    }

    // check the 'type' field in package.json
    let module_type = v["type"].as_str();

    if module_type.is_some() {
        if module_type.unwrap() == "module" {
            package_details.module_support.esm_type = true;
        } else if module_type.unwrap() == "commonjs" {
            package_details.module_support.cjs_type = true
        }
    }

    // check the 'module' field in package.json
    let module_field = v["module"].as_str();

    if module_field.is_some() {
        package_details.module_support.esm_partial = true;
    }

    // check the 'exports' field in package.json
    let exports = v["exports"].as_object();
    if exports.is_some() {
        let export_module_support = parse_exports(exports.unwrap());

        if export_module_support.esm_exports {
            package_details.module_support.esm_exports = true
        };
        if export_module_support.cjs_exports {
            package_details.module_support.cjs_exports = true
        };
    }

    return package_details;
}

fn dir_handler(entry: &DirEntry) -> PackageValidation {
    let path = entry.path();
    let file_name = entry.file_name();
    let mut package_validation = PackageValidation {
        is_package: false,
        package_details: PackageDetails {
            name: "".to_string(),
            version: "".to_string(),
            module_support: ModuleSupport {
                esm_main_mjs: false,
                esm_type: false,
                esm_exports: false,
                esm_partial: false,
                cjs_type: false,
                cjs_exports: false,
            },
        },
    };

    if file_name == "package.json" {
        package_validation.is_package = true;
        let contents = fs::read_to_string(path).expect("Unable to read file {path}");

        let v: Value = serde_json::from_str(&contents).expect("Unable to parse JSON");

        package_validation.package_details = parse_package(v);
        if package_validation.package_details.name == "" {
            package_validation.package_details.name =
                entry.path().parent().unwrap().display().to_string();
        }
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
