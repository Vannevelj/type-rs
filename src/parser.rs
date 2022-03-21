use log::{debug, trace};
use rslint_core::autofix::Fixer;
use rslint_errors::Span;
use rslint_parser::{
    ast::{Expr, FnDecl, FnExpr, Name, ObjectPatternProp, ParameterList, Pattern},
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
                for param in param_list.parameters().into_iter() {
                    debug!("Updating pattern");
                    update_pattern(&param, None, &mut fixer);
                }
            }
            _ => continue,
        }
    }

    fixer.apply()
}

fn update_pattern(pattern: &Pattern, type_annotation: Option<&str>, fixer: &mut Fixer) {
    for child in pattern.syntax().children() {
        trace!("child: {child:?}");
    }

    match pattern {
        Pattern::SinglePattern(single) if single.ty().is_none() => {
            trace!("single: {single:?}");
            if let Some(span) = single.name().map(|name| name.range()) {
                fixer.insert_after(span, format!(": {}", type_annotation.unwrap_or("any")));
            }
        }
        Pattern::RestPattern(_) => todo!(),
        Pattern::AssignPattern(assign) if assign.ty().is_none() => {
            // FIXME: AssignPattern.key() returns None so we work around it by querying the children instead. Should be Pattern::SinglePattern
            let expression_type = Some(get_type_from_expression(assign.value()));
            if let Some(name) = assign.syntax().child_with_ast::<Name>() {
                fixer.insert_after(
                    name.range(),
                    format!(": {}", expression_type.unwrap_or("any")),
                );
            }
        }
        Pattern::ObjectPattern(obj) if obj.ty().is_none() => {
            fixer.insert_after(
                obj.range(),
                format!(": {}", type_annotation.unwrap_or("any")),
            );
        }
        Pattern::ArrayPattern(array) => {
            debug!("array pattern: {:?}", array.text());
        }
        Pattern::ExprPattern(_) => todo!(),
        _ => (),
    }
}

fn get_type_from_expression<'a>(expr: Option<Expr>) -> &'a str {
    trace!("expr: {expr:?}");
    match expr {
        Some(Expr::ArrayExpr(_)) => "any[]",
        _ => "any"

        // Expr::ArrowExpr(_) => todo!(),
        // Expr::Literal(_) => todo!(),
        // Expr::Template(_) => todo!(),
        // Expr::NameRef(_) => todo!(),
        // Expr::ThisExpr(_) => todo!(),
        // Expr::ObjectExpr(_) => todo!(),
        // Expr::GroupingExpr(_) => todo!(),
        // Expr::BracketExpr(_) => todo!(),
        // Expr::DotExpr(_) => todo!(),
        // Expr::NewExpr(_) => todo!(),
        // Expr::CallExpr(_) => todo!(),
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
    fn add_types_array_function_default_value() {
        compare("function foo(a = []) {}", "function foo(a: any[] = []) {}");
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
}
