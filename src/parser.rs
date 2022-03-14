use log::error;
use swc_common::{SourceFile, Span};
use swc_ecma_ast::{
    Decl, EsVersion, FnDecl, Module, ModuleItem, Param, Pat, Stmt, TsKeywordType,
    TsKeywordTypeKind, TsType, TsTypeAnn,
};
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

pub fn add_types(module: &mut Module) -> String {
    for module_item in &mut module.body {
        match module_item {
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(ref mut function))) => {
                update_function_declaration(function)
            }
            _ => println!("not a statement"),
        }
    }

    update_module(module)
}

fn update_function_declaration(declaration: &mut FnDecl) {
    for param in &mut declaration.function.params {
        update_param(param);
    }
}

fn update_param(param: &mut Param) {
    update_pat(&mut param.pat);
}

fn update_pat(pat: &mut Pat) {
    match pat {
        Pat::Ident(found) => {
            let any_keyword = TsKeywordType {
                span: Span::default(),
                kind: TsKeywordTypeKind::TsAnyKeyword,
            };

            let type_annotation: TsTypeAnn = TsTypeAnn {
                span: Span::default(),
                type_ann: Box::new(TsType::TsKeywordType(any_keyword)),
            };

            found.type_ann = Some(type_annotation);
        }
        _ => todo!(),
    }
}

fn update_module(module: &Module) -> String {
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
            cm,
            wr,
        };

        emitter.emit_module(module).expect("Failed to emit module");
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

        let mut module = parse(&file);
        let result = add_types(&mut module);

        assert_eq!("function foo(a: any) {}\n", result);
    }
}
