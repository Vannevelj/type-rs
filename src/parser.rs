use std::sync::Arc;
use rslint_core::autofix::Fixer;
use rslint_errors::Span;
use rslint_parser::{parse_text, SyntaxNodeExt, AstNode};

pub fn add_types(contents: String) -> String {
    let parse = parse_text(contents.as_str(), 0);
    let ast = parse.syntax();
    let arc = Arc::from(contents);
    let mut fixer = Fixer::new(arc);

    for descendant in ast.descendants() {
        match descendant.kind() {
            rslint_parser::SyntaxKind::VAR_DECL => {
                let declaration = descendant.to::<rslint_parser::ast::VarDecl>();
                let declarators = declaration.declared();
                for declarator in declarators {
                    match declarator.pattern().unwrap() {
                        rslint_parser::ast::Pattern::SinglePattern(single) => {
                            let span = single.name().unwrap().syntax().as_range();
                            fixer.insert_after(span, ": any");                            
                        },
                        _ => continue
                    };
                }
            },
            _ => continue
        }
    }

    fixer.apply()
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

    #[test]
    fn add_types_preserves_comments() {
        compare("// hello", "// hello");
    }
}
