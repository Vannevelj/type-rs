use log::error;
use swc_common::SourceFile;
use swc_ecma_ast::{Decl, FnDecl, Module, Param, Pat};
use swc_ecma_codegen::{
    text_writer::{self, JsWriter},
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

pub fn add_types(module: Module) -> String {
    for module_item in module.body {
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

    render_module(module)
}

fn render_function_declaration(declaration: &FnDecl) {
    for param in &declaration.function.params {
        render_param(param);
    }
}

fn render_param(param: &Param) {
    render_pat(param.pat);
}

fn render_pat(pat: Pat) -> Pat {
    match pat.ident() {
        Some(found) => found.type_ann = Some("any"),
        None => todo!(),
    };

    pat
}

fn render_module(module: Module) -> String {
    let mut buf = vec![];
    {
        let mut wr = Box::new(JsWriter::with_target(
            self.cm.clone(),
            "\n",
            &mut buf,
            if source_map.enabled() {
                Some(&mut src_map_buf)
            } else {
                None
            },
            target,
        )) as Box<dyn WriteJs>;

        if minify {
            wr = Box::new(text_writer::omit_trailing_semi(wr));
        }

        let mut emitter = Emitter {
            cfg: Config { minify },
            comments,
            cm: self.cm.clone(),
            wr,
        };

        node.emit_with(&mut emitter)
            .context("failed to emit module")?;
    }
    // Invalid utf8 is valid in javascript world.
    String::from_utf8(buf).expect("invalid utf8 character detected")
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
        let result = add_types(module);

        assert_eq!("function foo(a: any) {}", result);
    }
}
