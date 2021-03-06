use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::{fs, thread};
use structopt::StructOpt;
use type_rs::options::Options;
use type_rs::parser::add_types;

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = Options::from_args();
    info!("Starting now at {:?}", &args.path);

    traverse_directories(args.path);

    info!("Finished conversion!")
}

fn traverse_directories(path: PathBuf) {
    // We use metadata since path::is_file() coerces an error into false
    let metadata = match fs::metadata(path.clone()) {
        Ok(m) => m,
        Err(err) => {
            warn!("Unable to read the metadata for {:?}: {}", path, err);
            return;
        }
    };

    if let Some(full_path) = path.as_os_str().to_str() {
        if full_path.contains("node_modules") {
            return;
        }
    }

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

            info!("Processing {:?}", path);

            thread::spawn(move || handle_file(path, file_name, target_extension));
            return;
        }
    }

    debug!("Diving into new directory: {:?}", path);
    for directory in fs::read_dir(path).unwrap().flatten() {
        traverse_directories(directory.path().to_path_buf());
    }
}

fn handle_file(path: PathBuf, file_name: String, extension: &str) {
    match fs::read_to_string(path.clone()) {
        Ok(contents) => {
            if contents.contains("@flow") {
                warn!("Skipped {path:?} due to Flow");
                return;
            }

            let extension = if contents.contains("import React") {
                "tsx"
            } else {
                extension
            };

            let new_source = add_types(contents);
            let new_path = path.with_file_name(format!("{file_name}.{extension}"));
            debug!("Writing new file at {new_path:?}");
            fs::write(new_path, new_source).expect("Unable to write file");
            fs::remove_file(path).expect("Failed to delete file");
        }
        Err(error) => {
            error!("Unable to load file {file_name}: {error}");
        }
    }
}
