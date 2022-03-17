use log::{error, info};
use swc_common::sync::Lrc;
use swc_common::{SourceFile, SourceMap, Span};
use swc_ecma_ast::{
    Decl, EsVersion, Expr, FnDecl, ModuleItem, Param, Pat, Program, Stmt, TsArrayType,
    TsKeywordType, TsKeywordTypeKind, TsType, TsTypeAnn, VarDecl,
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
                    ModuleItem::ModuleDecl(_) => (),
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
                Decl::Class(_) => (),
                Decl::Fn(ref mut function) => update_function_declaration(function),
                Decl::Var(ref mut variable) => {
                    info!("\nvar declr");
                    update_variable_declaration(variable)
                }
                Decl::TsInterface(_) => (),
                Decl::TsTypeAlias(_) => (),
                Decl::TsEnum(_) => (),
                Decl::TsModule(_) => (),
            }
        }
        Stmt::Block(_) => (),
        Stmt::Empty(_) => (),
        Stmt::Debugger(_) => (),
        Stmt::With(_) => (),
        Stmt::Return(_) => (),
        Stmt::Labeled(_) => (),
        Stmt::Break(_) => (),
        Stmt::Continue(_) => (),
        Stmt::If(_) => (),
        Stmt::Switch(_) => (),
        Stmt::Throw(_) => (),
        Stmt::Try(_) => (),
        Stmt::While(_) => (),
        Stmt::DoWhile(_) => (),
        Stmt::For(_) => (),
        Stmt::ForIn(_) => (),
        Stmt::ForOf(_) => (),
        Stmt::Expr(e) => {
            info!("\nexprstmt: {e:?}");
        }
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
    for declarator in &mut declaration.decls {
        info!("var_declarator: {declarator:?}");
        let type_ann = declarator
            .init
            .as_mut()
            .map(|initializer| get_type_from_expression(&mut *initializer));
        update_pat(&mut declarator.name, type_ann)
    }
}

fn update_param(param: &mut Param) {
    update_pat(&mut param.pat, None);
}

fn update_pat(pat: &mut Pat, with_type: Option<TsTypeAnn>) {
    let with_type = with_type.unwrap_or_else(create_any_type);
    info!("pat: {pat:?}");
    match pat {
        Pat::Ident(ident) => {
            if ident.type_ann.is_none() {
                ident.type_ann = Some(with_type)
            }
        }
        Pat::Array(_) => (),
        Pat::Rest(_) => (),
        Pat::Object(_) => (),
        Pat::Assign(assign) => {
            let type_annotation = get_type_from_expression(&mut *assign.right);
            update_pat(&mut assign.left, Some(type_annotation));
        }
        Pat::Invalid(_) => (),
        Pat::Expr(_) => (),
    }
}

fn get_type_from_expression(expr: &mut Expr) -> TsTypeAnn {
    match expr {
        Expr::Array(ref array) => create_array(create_any_type()),
        //Expr::Lit(_) => (),
        _ => create_any_type(),
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
        Syntax::Typescript(Default::default()),
        EsVersion::Es5,
        StringInput::from(&*file),
        None,
    )
}

fn create_any_type() -> TsTypeAnn {
    let any_keyword = TsKeywordType {
        span: Span::default(),
        kind: TsKeywordTypeKind::TsAnyKeyword,
    };

    TsTypeAnn {
        span: Span::default(),
        type_ann: Box::new(TsType::TsKeywordType(any_keyword)),
    }
}

fn create_array(element_type: TsTypeAnn) -> TsTypeAnn {
    let array_keyword = TsArrayType {
        span: Span::default(),
        elem_type: element_type.type_ann,
    };

    TsTypeAnn {
        span: Span::default(),
        type_ann: Box::new(TsType::TsArrayType(array_keyword)),
    }
}

#[cfg(test)]
mod tests {
    use swc_common::FileName;

    use super::*;

    fn compare(input: &str, expected_output: &str) {
        // env_logger::init_from_env(
        //     env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
        // );

        let cm: Lrc<SourceMap> = Default::default();
        let file = cm.new_source_file(FileName::Custom("test.js".into()), input.into());

        let mut module = parse(&file);
        let actual_output = add_types(&mut module, cm);

        assert_eq!(format!("{expected_output}\n"), actual_output);
    }

    #[test]
    fn add_types_function() {
        compare("function foo(a) {}", "function foo(a: any) {}");
    }

    #[test]
    fn add_types_function_default_value() {
        compare("function foo(a = 5) {}", "function foo(a: any = 5) {}");
    }

    #[test]
    fn add_types_function_multi_param() {
        compare(
            "function foo(a, b, c) {}",
            "function foo(a: any, b: any, c: any) {}",
        );
    }

    #[test]
    fn add_types_variable_no_initializer() {
        compare("let x;", "let x: any;");
    }

    #[test]
    fn add_types_variable_let() {
        compare("let x = 5;", "let x: any = 5;");
    }

    #[test]
    fn add_types_variable_const() {
        compare("const x = 5;", "const x: any = 5;");
    }

    #[test]
    fn add_types_variable_var() {
        compare("var x = 5;", "var x: any = 5;");
    }

    #[test]
    fn add_types_variable_multi() {
        compare("let x = 5, y = 6;", "let x: any = 5, y: any = 6;");
    }

    #[test]
    fn add_types_variable_in_function() {
        compare(
            "function foo() { let x = 5; }",
            "function foo() {
    let x: any = 5;
}",
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
}",
        );
    }

    #[test]
    fn add_types_array() {
        compare("const x = [];", "const x: any[] = [];");
    }

    #[test]
    fn add_types_array_function_default_value() {
        compare("function foo(a = []) {}", "function foo(a: any[] = []) {}");
    }

    #[test]
    fn add_types_preexisting_type() {
        compare("let x: number = 5;", "let x: number = 5;");
    }
}
