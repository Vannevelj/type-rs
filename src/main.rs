mod parser;

use std::path::Path;
use std::{fs, path::PathBuf};

use log::{debug, error, info, warn};
use swc_common::sync::Lrc;
use swc_common::SourceMap;

use crate::parser::{add_types, parse};

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    let path = Path::new("C:\\source\\hudl-videospa\\src\\client-app\\app").to_path_buf();
    traverse_directories(&path);
}

fn traverse_directories(path: &Path) {
    // We use metadata since path::is_file() coerces an error into false
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(err) => {
            warn!("Unable to read the metadata for {:?}: {}", path, err);
            return;
        }
    };

    if metadata.is_file() {
        if let Some(file_name) = path.file_stem() {
            let file_name = file_name
                .to_os_string()
                .into_string()
                .expect("Failed to convert file path");
            let target_extension = match path.extension().and_then(|ext| ext.to_str()) {
                Some("js") => "ts",
                Some("jsx") => "tsx",
                _ => return,
            };

            debug!("Processing {:?}", file_name);
            let cm: Lrc<SourceMap> = Default::default();

            return match cm.load_file(path) {
                Ok(source_file) => {
                    let mut program = parse(&source_file);
                    let new_source = add_types(&mut program, cm);
                    let new_path = path.with_file_name(format!("{file_name}.{target_extension}"));
                    info!("Writing new file at {new_path:?}");
                    fs::write(new_path, new_source).expect("Unable to write file");
                    fs::remove_file(path).expect("Failed to delete file");
                }
                Err(_) => {
                    error!("Unable to load file: {file_name}");
                }
            };
        }
    }

    debug!("Diving into new directory: {:?}", path);
    for entry in fs::read_dir(path).unwrap() {
        if let Ok(directory) = entry {
            traverse_directories(&directory.path());
        }
    }
}
