mod parser;

use swc_common::sync::Lrc;
use swc_common::{
    FileName, SourceMap,
};

use crate::parser::parse;

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let cm: Lrc<SourceMap> = Default::default();

    // Real usage
    // let fm = cm
    //     .load_file(Path::new("test.js"))
    //     .expect("failed to load test.js");
    let file = cm.new_source_file(
        FileName::Custom("test.js".into()),
        "function foo() {}".into(),
    );    

    let module = parse(&file);
    println!("Module: {module:?}");
}