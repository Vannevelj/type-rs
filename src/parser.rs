use rslint_core::autofix::Fixer;
use rslint_errors::Span;
use rslint_parser::{
    ast::{FnDecl, Pattern, VarDecl},
    parse_text, AstNode, SyntaxKind, SyntaxNodeExt, SyntaxNode,
};
use std::sync::Arc;

pub fn add_types(contents: String) -> String {
    let parse = parse_text(contents.as_str(), 0);
    let ast = parse.syntax();
    let arc = Arc::from(contents);
    let mut fixer = Fixer::new(arc);
    print_ast(&ast);

    for descendant in ast.descendants() {
        match descendant.kind() {
            SyntaxKind::VAR_DECL => {
                let declaration = descendant.to::<VarDecl>();
                for declarator in declaration.declared() {
                    if let Some(pat) = declarator.pattern() {
                        update_pattern(&pat, &mut fixer);
                    }
                }
            }
            SyntaxKind::FN_DECL => {
                let declaration = descendant.to::<FnDecl>();
                for param in declaration
                    .parameters()
                    .into_iter()
                    .flat_map(|pl| pl.parameters())
                {
                    update_pattern(&param, &mut fixer);
                }
            }
            _ => continue,
        }
    }

    fixer.apply()
}

fn update_pattern(pattern: &Pattern, fixer: &mut Fixer) {
    match pattern {
        Pattern::SinglePattern(single) => {
            println!("single: {single:?}");
            let span = single.name().unwrap().syntax().as_range();
            fixer.insert_after(span, ": any");
        }
        Pattern::RestPattern(_) => todo!(),
        Pattern::AssignPattern(assign) => {
            println!("assign: {assign:?}");
            println!("value: {:?}", assign.value());
            println!("ty: {:?}", assign.ty());
            println!("eq_token: {:?}", assign.eq_token());
            println!("colon_token: {:?}", assign.colon_token());
            println!("decorators: {:?}", assign.decorators());
            println!("text: {:?}", assign.text());
            println!("key: {:?}", assign.key());
            
            if let Some(pat) = assign.key() {
                println!("key: {pat:?}");
                update_pattern(&pat, fixer);
            }
        }
        Pattern::ObjectPattern(_) => todo!(),
        Pattern::ArrayPattern(_) => todo!(),
        Pattern::ExprPattern(_) => todo!(),
    }
}

fn print_ast(root: &SyntaxNode) {
    fn write_node(node: &SyntaxNode, depth: usize) {
        let name = node.readable_stmt_name();
        let spaces = " ".repeat(depth);

        println!("{spaces}{name} ({:?}) [{:?}-{:?}]", &node.kind(), &node.text_range().start(), &node.text_range().end());

        for child in node.children() {
            write_node(&child, depth + 1);
        }
    }

    write_node(root, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(input: &str, expected_output: &str) {
        // env_logger::init_from_env(
        //     env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "trace"),
        // );
        let output = add_types(String::from(input));
        assert_eq!(expected_output, output);
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
            "function foo() { let x: any = 5; }",
        );
    }

    #[test]
    fn add_types_whitespace() {
        compare("
function foo(
    a,
    b,
    c) 
{
    console.log(test);
}","
function foo(
    a: any,
    b: any,
    c: any) 
{
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

    #[test]
    fn add_types_preserves_comments() {
        compare("// hello", "// hello");
    }
}
