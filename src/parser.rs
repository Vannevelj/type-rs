use log::error;
use swc_common::SourceFile;
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, StringInput, Syntax, Parser};

pub fn parse(file: &SourceFile) -> Module {
    let lexer = create_lexer(&file);
    let mut parser = Parser::new_from(lexer);
    parser
        .parse_module()
        .map_err(|e| {
            error!("Error: {e:?}");
        })
        .expect("failed to parse module")
}

fn create_lexer(file: &SourceFile) -> Lexer<StringInput> {
    Lexer::new(
        Syntax::Es(Default::default()),
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*file),
        None,
    )
}