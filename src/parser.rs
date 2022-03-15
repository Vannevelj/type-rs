use log::{error, info};
use swc_common::sync::Lrc;
use swc_common::{SourceFile, SourceMap, Span};
use swc_ecma_ast::{
    Decl, EsVersion, FnDecl, Module, ModuleItem, Param, Pat, Program, Script, Stmt, TsKeywordType,
    TsKeywordTypeKind, TsType, TsTypeAnn,
};
use swc_ecma_codegen::{
    text_writer::{JsWriter, WriteJs},
    Config, Emitter,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

pub fn parse(file: &SourceFile) -> Program {
    let lexer = create_lexer(file);
    let mut parser = Parser::new_from(lexer);
    parser
        .parse_program()
        .map_err(|e| {
            error!("Error: {e:?}");
        })
        .expect("failed to parse module")
}

pub fn add_types(module: &mut Program, cm: Lrc<SourceMap>) -> String {
    fn handle_statement(stmt: &mut Stmt) {
        match stmt {
            Stmt::Decl(Decl::Fn(ref mut function)) => {
                println!("Updating function declaration as script");
                update_function_declaration(function)
            }
            _ => error!("not a statement"),
        }
    }

    match module {
        Program::Module(module) => {
            for module_item in &mut module.body {
                info!("Found module item: {module_item:?}");
                match module_item {
                    ModuleItem::Stmt(stmt) => {
                        handle_statement(stmt);
                    }
                    _ => error!("not a statement"),
                }
            }
        }
        Program::Script(script) => {
            for statement in &mut script.body {
                info!("Found statement: {statement:?}");
                handle_statement(statement);
            }
        }
    }
    
    update_program(module, cm)
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
        Pat::Ident(ident) => {
            let any_keyword = TsKeywordType {
                span: Span::default(),
                kind: TsKeywordTypeKind::TsAnyKeyword,
            };

            let type_annotation: TsTypeAnn = TsTypeAnn {
                span: Span::default(),
                type_ann: Box::new(TsType::TsKeywordType(any_keyword)),
            };

            ident.type_ann = Some(type_annotation);
        }
        _ => todo!(),
    }
}

fn update_program(program: &Program, cm: Lrc<SourceMap>) -> String {
    let mut buf = vec![];
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

        emitter.emit_program(program).expect("Failed to emit program");
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
    use swc_common::FileName;

    use super::*;

    fn compare(input: &str, output: &str) {
        let cm: Lrc<SourceMap> = Default::default();
        let file = cm.new_source_file(FileName::Custom("test.js".into()), input.into());

        let mut module = parse(&file);
        let result = add_types(&mut module, cm);

        assert_eq!(output, result);
    }

    #[test]
    fn add_types_function() {
        compare("function foo(a) {}", "function foo(a: any) {}\n");
        compare(
            "function foo(a, b, c) {}",
            "function foo(a: any, b: any, c: any) {}\n",
        );
    }

    #[test]
    fn add_types_variable() {
        //compare("let x = 5;", "let x: number = 5;\n");
        // compare("const x = 5;", "const x: number = 5;\n");
        // compare("var x = 5;", "var x: number = 5;\n");
        compare(
            "function foo() { let x = 5; }",
            "function foo() { 
    let x: number = 5; 
}\n",
        );
    }

    #[test]
    fn add_types_whitespace() {
        compare(
            "function foo(
            a,
            b,
            c
        ) {
            console.log(test);
        }",
            "function foo(a: any, b: any, c: any) {
    console.log(test);
}\n",
        );
    }
}
