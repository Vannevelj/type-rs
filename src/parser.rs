use log::error;
use swc_common::{SourceFile};
use swc_ecma_ast::{Decl, FnDecl, Module, Param, Pat, EsVersion};
use swc_ecma_codegen::{
    text_writer::{JsWriter, WriteJs},
    Config, Emitter,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

pub fn parse(file: &SourceFile) -> Module {
    let lexer = create_lexer(file);
    let mut parser = Parser::new_from(lexer);
    parser
        .parse_module()
        .map_err(|e| {
            error!("Error: {e:?}");
        })
        .expect("failed to parse module")
}

pub fn add_types(module: &Module) -> String {
    for module_item in &module.body {
        println!("module_item: {module_item:?}");
        match module_item.as_stmt() {
            Some(statement) => match statement.as_decl() {
                Some(Decl::Fn(function)) => {
                    render_function_declaration(function);
                    print!("found function: {function:?}")
                }
                _ => println!("not a function"),
            },
            None => println!("not a statement"),
        }
    }

    render_module(&module)
}

fn render_function_declaration(declaration: &FnDecl) {
    for param in &declaration.function.params {
        render_param(param);
    }
}

fn render_param(param: &Param) {
    render_pat(&param.pat);
}

fn render_pat(pat: &Pat) {
    match pat.clone().ident() {
        Some(found) => {},
        None => todo!(),
    }
}

fn render_module(module: &Module) -> String {
    let mut buf = vec![];
    let cm: swc_common::sync::Lrc<swc_common::SourceMap> = Default::default();
    {
        let wr = Box::new(JsWriter::with_target(
            cm.clone(),
            "\n",
            &mut buf,
            None,
            EsVersion::Es5,
        )) as Box<dyn WriteJs>;

        let mut emitter = Emitter {
            cfg: Config { minify: false },
            comments: None,
            cm: cm.clone(),
            wr,
        };

        emitter.emit_module(&module).expect("Failed to emit module");
    }

    String::from_utf8(buf).expect("invalid utf8 character detected")
}

fn create_lexer(file: &SourceFile) -> Lexer<StringInput> {
    Lexer::new(
        Syntax::Es(Default::default()),
        EsVersion::Es5,
        StringInput::from(&*file),
        None,
    )
}

#[cfg(test)]
mod tests {
    use swc_common::sync::Lrc;
    use swc_common::{FileName, SourceMap};

    use super::*;

    #[test]
    fn add_types_function() {
        let cm: Lrc<SourceMap> = Default::default();
        let file = cm.new_source_file(
            FileName::Custom("test.js".into()),
            "function foo(a) {}".into(),
        );

        let module = parse(&file);
        let result = add_types(&module);

        assert_eq!("function foo(a: any) {}", result);
    }
}
