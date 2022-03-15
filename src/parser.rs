use log::{error, info};
use swc_common::sync::Lrc;
use swc_common::{SourceFile, SourceMap, Span};
use swc_ecma_ast::{
    Decl, EsVersion, FnDecl, ModuleItem, Param, Pat, Program, Stmt, TsKeywordType,
    TsKeywordTypeKind, TsType, TsTypeAnn, VarDecl,
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

pub fn add_types(program: &mut Program, cm: Lrc<SourceMap>) -> String {
    match program {
        Program::Module(module) => {
            for module_item in &mut module.body {
                info!("\nFound module item: {module_item:?}");
                match module_item {
                    ModuleItem::Stmt(stmt) => {
                        handle_statement(stmt);
                    }
                    _ => error!("not a statement"),
                }
            }
        }
        Program::Script(script) => {
            info!("\nscript: {script:?}");
            for statement in &mut script.body {
                info!("\nFound statement: {statement:?}");
                handle_statement(statement);
            }
        }
    }

    update_program(program, cm)
}

fn handle_statement(stmt: &mut Stmt) {
    info!("\nstmt: {stmt:?}");

    match stmt {
        Stmt::Decl(declaration) => {
            info!("\ndeclaration: {declaration:?}");
            match declaration {
                Decl::Class(_) => todo!(),
                Decl::Fn(ref mut function) => update_function_declaration(function),
                Decl::Var(ref mut variable) => {
                    info!("\nvar declr");
                    update_variable_declaration(variable)
                }
                Decl::TsInterface(_) => todo!(),
                Decl::TsTypeAlias(_) => todo!(),
                Decl::TsEnum(_) => todo!(),
                Decl::TsModule(_) => todo!(),
            }
        }
        _ => error!("not a statement"),
    }
}

fn update_function_declaration(declaration: &mut FnDecl) {
    for param in &mut declaration.function.params {
        update_param(param);
    }

    if let Some(body) = &mut declaration.function.body {
        for stmt in &mut body.stmts {
            handle_statement(stmt);
        }
    }
}

fn update_variable_declaration(declaration: &mut VarDecl) {
    for var_declaration in &mut declaration.decls {
        info!("var_declarator: {var_declaration:?}");
        update_pat(&mut var_declaration.name)
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

        emitter
            .emit_program(program)
            .expect("Failed to emit program");
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
    }

    #[test]
    fn add_types_function_multi_param() {
        compare(
            "function foo(a, b, c) {}",
            "function foo(a: any, b: any, c: any) {}\n",
        );
    }

    #[test]
    fn add_types_variable_let() {
        compare("let x = 5;", "let x: any = 5;\n");
    }

    #[test]
    fn add_types_variable_const() {
        compare("const x = 5;", "const x: any = 5;\n");
    }

    #[test]
    fn add_types_variable_var() {
        compare("var x = 5;", "var x: any = 5;\n");
    }

    #[test]
    fn add_types_variable_multi() {
        compare("let x = 5, y = 6;", "let x: any = 5, y: any = 6;\n");
    }

    #[test]
    fn add_types_variable_in_function() {
        compare(
            "function foo() { let x = 5; }",
            "function foo() {
    let x: any = 5;
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
