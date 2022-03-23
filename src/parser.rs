use log::{debug, trace};
use rslint_core::autofix::Fixer;
use rslint_parser::{
    ast::{Declarator, Expr, ExprOrSpread, ForStmtInit, LiteralKind, Name, ParameterList, Pattern},
    parse_with_syntax, AstNode, Syntax, SyntaxKind, SyntaxNode, SyntaxNodeExt,
};
use std::sync::Arc;

pub fn add_types(contents: String) -> String {
    let syntax = Syntax::default().typescript();
    let parse = parse_with_syntax(contents.as_str(), 0, syntax);
    let ast = parse.syntax();
    let arc = Arc::from(contents);
    let mut fixer = Fixer::new(arc);
    print_ast(&ast);

    for descendant in ast.descendants() {
        match descendant.kind() {
            SyntaxKind::PARAMETER_LIST => {
                let param_list = descendant.to::<ParameterList>();
                for param in param_list.parameters() {
                    update_pattern(&param, &mut fixer);
                }
            }
            SyntaxKind::DECLARATOR => {
                let declarator = descendant.to::<Declarator>();
                if declarator
                    .syntax()
                    .ancestors()
                    .any(|ancestor| ancestor.is::<ForStmtInit>())
                {
                    continue;
                }

                debug!("declarator.value: {:?}", declarator.value());
                debug!("declarator.pattern: {:?}", declarator.pattern());

                if let Some(ref pattern) = declarator.pattern() {
                    match declarator.value() {
                        None => update_pattern(pattern, &mut fixer),
                        Some(Expr::Literal(literal)) if literal.is_null() => {
                            update_pattern(pattern, &mut fixer)
                        }
                        Some(Expr::NameRef(name_ref)) if name_ref.text() == "undefined" => {
                            update_pattern(pattern, &mut fixer)
                        }
                        _ => (),
                    }
                }
            }
            _ => continue,
        }
    }

    fixer.apply()
}

fn update_pattern(pattern: &Pattern, fixer: &mut Fixer) {
    for child in pattern.syntax().children() {
        trace!("child: {child:?}");
    }

    match pattern {
        Pattern::SinglePattern(single) if single.ty().is_none() => {
            trace!("single: {single:?}");
            if let Some(span) = single.name().map(|name| name.range()) {
                if let Some(type_annotation) = get_type_from_expression(None) {
                    fixer.insert_after(span, format!(": {}", type_annotation));
                }                
            }
        }
        Pattern::RestPattern(_) => todo!(),
        Pattern::AssignPattern(assign) if assign.ty().is_none() => {
            // FIXME: AssignPattern.key() returns None so we work around it by querying the children instead. Should be Pattern::SinglePattern
            if let Some(type_annotation) = get_type_from_expression(assign.value()) {
                if let Some(name) = assign.syntax().child_with_ast::<Name>() {
                    fixer.insert_after(name.range(), format!(": {}", type_annotation));
                }
            }            
        }
        Pattern::ObjectPattern(obj) if obj.ty().is_none() => {
            if let Some(type_annotation) = get_type_from_expression(None) {
                fixer.insert_after(obj.range(), format!(": {}", type_annotation));
            }            
        }
        Pattern::ArrayPattern(array) => {
            debug!("array pattern: {:?}", array.text());
        }
        Pattern::ExprPattern(_) => todo!(),
        _ => (),
    }
}

fn get_type_from_expression(expr: Option<Expr>) -> Option<String> {
    trace!("expr: {expr:?}");
    match expr {
        Some(Expr::ArrayExpr(array)) => {
            let default_return = Some(String::from("any[]"));
            let mut found_type = None;
            for element in array.elements() {
                if let ExprOrSpread::Expr(expr) = element {
                    let expression_type = get_type_from_expression(Some(expr));
                    match expression_type {
                        Some(element_type) => {
                            match found_type {
                                // FIXME: we can make this smarter by constructing a union type, e.g. `(string | number)[]`
                                Some(t) if t != element_type => return default_return,
                                _ => found_type = Some(format!("{}[]", element_type))
                            }
                        }
                        None => return None
                    }
                    
                }
            }

            found_type.or(default_return)
        },
        Some(Expr::Literal(literal)) => {
            match literal.kind() {
                LiteralKind::Number(_) => Some(String::from("number")),
                LiteralKind::BigInt(_) => Some(String::from("BigInt")),
                LiteralKind::String => Some(String::from("string")),
                LiteralKind::Null => Some(String::from("any")),
                LiteralKind::Bool(_) => Some(String::from("boolean")),
                LiteralKind::Regex => Some(String::from("RegExp")),
            }
        }
        Some(Expr::CallExpr(_)) => None,
        Some(Expr::NewExpr(_)) => None,
        _ => Some(String::from("any"))

        // Expr::ArrowExpr(_) => todo!(),
        // Expr::Template(_) => todo!(),
        // Expr::NameRef(_) => todo!(),
        // Expr::ThisExpr(_) => todo!(),
        // Expr::ObjectExpr(_) => todo!(),
        // Expr::GroupingExpr(_) => todo!(),
        // Expr::BracketExpr(_) => todo!(),
        // Expr::DotExpr(_) => todo!(),
        // ,
        
        // Expr::UnaryExpr(_) => todo!(),
        // Expr::BinExpr(_) => todo!(),
        // Expr::CondExpr(_) => todo!(),
        // Expr::AssignExpr(_) => todo!(),
        // Expr::SequenceExpr(_) => todo!(),
        // Expr::FnExpr(_) => todo!(),
        // Expr::ClassExpr(_) => todo!(),
        // Expr::NewTarget(_) => todo!(),
        // Expr::ImportMeta(_) => todo!(),
        // Expr::SuperCall(_) => todo!(),
        // Expr::ImportCall(_) => todo!(),
        // Expr::YieldExpr(_) => todo!(),
        // Expr::AwaitExpr(_) => todo!(),
        // Expr::PrivatePropAccess(_) => todo!(),
        // Expr::TsNonNull(_) => todo!(),
        // Expr::TsAssertion(_) => todo!(),
        // Expr::TsConstAssertion(_) => todo!(),
    }
}

fn print_ast(root: &SyntaxNode) {
    fn write_node(node: &SyntaxNode, depth: usize) {
        let name = node.readable_stmt_name();
        let spaces = " ".repeat(depth);

        trace!(
            "{spaces}{name} ({:?}) [{:?}-{:?}]",
            &node.kind(),
            &node.text_range().start(),
            &node.text_range().end()
        );

        for child in node.children() {
            write_node(&child, depth + 1);
        }
    }

    write_node(root, 0);
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "trace"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(input: &str, expected_output: &str) {
        let output = add_types(String::from(input));
        assert_eq!(expected_output, output);
    }

    #[test]
    fn add_types_function() {
        compare("function foo(a) {}", "function foo(a: any) {}");
    }

    #[test]
    fn add_types_function_default_value_number() {
        compare("function foo(a = 5) {}", "function foo(a: number = 5) {}");
    }

    #[test]
    fn add_types_function_default_value_string() {
        compare(
            "function foo(a = \"hey\") {}",
            "function foo(a: string = \"hey\") {}",
        );
    }

    #[test]
    fn add_types_function_default_value_object() {
        compare("function foo(a = {}) {}", "function foo(a: any = {}) {}");
    }

    #[test]
    fn add_types_function_default_value_array() {
        compare("function foo(a = []) {}", "function foo(a: any[] = []) {}");
    }

    #[test]
    fn add_types_function_default_value_array_string() {
        compare(
            "function foo(a = [\"s1\"]) {}",
            "function foo(a: string[] = [\"s1\"]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_array_number() {
        compare(
            "function foo(a = [1]) {}",
            "function foo(a: number[] = [1]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_array_mixed() {
        compare(
            "function foo(a = [\"s1\", 1]) {}",
            "function foo(a: any[] = [\"s1\", 1]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_array_mixed_null() {
        compare(
            "function foo(a = [1, null]) {}",
            "function foo(a: any[] = [1, null]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_null() {
        compare(
            "function foo(a = null) {}",
            "function foo(a: any = null) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_undefined() {
        compare(
            "function foo(a = undefined) {}",
            "function foo(a: any = undefined) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_regex() {
        compare(
            "function foo(a = /.*/) {}",
            "function foo(a: RegExp = /.*/) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_bigint_suffix() {
        compare(
            "function foo(a = 9007199254740991n) {}",
            "function foo(a: BigInt = 9007199254740991n) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_bigint_ctor() {
        compare(
            "function foo(a = BigInt(9007199254740991)) {}",
            "function foo(a = BigInt(9007199254740991)) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_bool() {
        compare(
            "function foo(a = true) {}",
            "function foo(a: boolean = true) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_date() {
        compare(
            "function foo(a = new Date()) {}",
            "function foo(a = new Date()) {}",
        );
    }

    #[test]
    fn add_types_function_multi_param() {
        compare(
            "function foo(a, b, c) {}",
            "function foo(a: any, b: any, c: any) {}",
        );
    }

    #[test]
    fn add_types_respects_whitespace() {
        compare(
            "
function foo(
    a,
    b,
    c) 
{
    /* hello comments */
    console.log(test);
}",
            "
function foo(
    a: any,
    b: any,
    c: any) 
{
    /* hello comments */
    console.log(test);
}",
        );
    }

    #[test]
    fn add_types_preexisting_type() {
        compare(
            "function foo(a: string[] = []) {}",
            "function foo(a: string[] = []) {}",
        );
    }

    #[test]
    fn add_types_preserves_comments() {
        compare("// hello", "// hello");
    }

    #[test]
    fn add_types_destructured_parameter() {
        compare(
            "function getRole({ permissions, user }) { }",
            "function getRole({ permissions, user }: any) { }",
        );
    }

    #[test]
    fn add_types_for_in() {
        compare(
            "
function foo() {          
    for (const key in {}) {

    }
}",
            "
function foo() {          
    for (const key in {}) {

    }
}",
        );
    }

    #[test]
    fn add_types_export_default_function() {
        compare(
            "export default function foo(route) { }",
            "export default function foo(route: any) { }",
        );
    }

    #[test]
    fn add_types_class_function() {
        compare(
            "
class ColorPicker {
  componentDidUpdate(prevProps, prevState) { }
}",
            "
class ColorPicker {
  componentDidUpdate(prevProps: any, prevState: any) { }
}",
        );
    }

    #[test]
    fn add_types_const_arrow_function() {
        compare(
            "const mapStateToProps = (state, props) => { }",
            "const mapStateToProps = (state: any, props: any) => { }",
        );
    }

    #[test]
    fn add_types_lambda() {
        compare(
            "function foo() { sources.filter((v, k) => true; }",
            "function foo() { sources.filter((v: any, k: any) => true; }",
        );
    }

    #[test]
    fn add_types_functions_as_object_keys() {
        compare(
            "
function foo() {
  return {
    bar: (a, b) => {},
  };
};",
            "
function foo() {
  return {
    bar: (a: any, b: any) => {},
  };
};",
        );
    }

    #[test]
    fn add_types_variable_uninitialized_let() {
        compare("let test;", "let test: any;");
    }

    #[test]
    fn add_types_variable_uninitialized_const() {
        compare("const test;", "const test: any;");
    }

    #[test]
    fn add_types_variable_uninitialized_var() {
        compare("var test;", "var test: any;");
    }

    #[test]
    fn add_types_variable_uninitialized_multi() {
        compare("let test, test2;", "let test: any, test2: any;");
    }

    #[test]
    fn add_types_variable_initialized_ambiguous_null() {
        compare("let test = null;", "let test: any = null;");
    }

    #[test]
    fn add_types_variable_initialized_ambiguous_undefined() {
        compare("let test = undefined;", "let test: any = undefined;");
    }

    #[test]
    fn add_types_variable_initialized_array_variable_untouched() {
        compare("let test = [];", "let test = [];");
    }

    #[test]
    fn add_types_variable_initialized_variable_number_untouched() {
        compare("let test = 5;", "let test = 5;");
    }

    #[test]
    #[ignore = "rslint does not support parsing JSX"]
    fn add_types_callback_arg() {
        compare(
            "return (
<Component
    onClick={() => this.toggleSelection(entity)}
/>);",
            "return (
<Component
    onClick={() => this.toggleSelection(entity)}
/>);",
        );
    }
}
