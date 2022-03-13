use log::error;
use swc_common::SourceFile;
use swc_ecma_ast::{Module, Decl, FnDecl, Ident, Param, Pat};
use swc_ecma_parser::{lexer::Lexer, StringInput, Syntax, Parser};

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
    let mut result = String::from("");

    for module_item in module.body {
        println!("module_item: {module_item:?}");
        match module_item.as_stmt() {
            Some(statement) => {
                match statement.as_decl() {
                    Some(Decl::Fn(function)) => {
                        result += &render_function_declaration(function);
                        print!("found function: {function:?}")
                    }
                    _ => println!("not a function")
                }
            }
            None => println!("not a statement"),
        }
    }

    result
}

fn render_function_declaration(declaration: &FnDecl) -> String {
    let mut result = String::from("");
    result += "function ";
    result += &render_ident(&declaration.ident);
    result += "(";
    for param in &declaration.function.params {
        result += &render_param(param);
    }
    result += ")";
    result += " ";
    result += "{";
    result += "}";

    result
}

fn render_ident(ident: &Ident) -> String {
    let mut result = String::from("");
    result += &ident.sym.to_string();

    result
}

fn render_param(param: &Param) -> String {
    let mut result = String::from("");
    result += &render_pat(param.pat.clone());
    result += ": any";

    result
}

fn render_pat(pat: Pat) -> String {
    let mut result = String::from("");
    let rendered_ident = match pat.ident() {
        Some(found) => found.id.sym.to_string(),
        None => todo!(),
    };

    result += &rendered_ident;
    result
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
    use swc_common::{FileName, SourceMap};
    use swc_common::sync::Lrc;

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