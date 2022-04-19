use inflector::Inflector;
use log::{debug, trace};
use rslint_parser::{
    ast::{
        ArgList, ArrowExpr, CatchClause, ClassDecl, Constructor, Declarator, Expr, ExprOrSpread,
        ExprStmt, FnDecl, FnExpr, ForStmtInit, LiteralKind, Method, Name, ParameterList, Pattern,
    },
    parse_with_syntax, AstNode, Syntax, SyntaxKind, SyntaxNode, SyntaxNodeExt,
};
use std::collections::BTreeSet;

use crate::{
    text_editor::{TextEdit, TextEditor},
    type_definition::{gather_usages, NameWithType},
};

pub fn add_types(contents: String) -> String {
    let syntax = Syntax::default().typescript();
    let parse = parse_with_syntax(contents.as_str(), 0, syntax);
    let ast = parse.syntax();
    let mut fixer = TextEditor::load(contents);
    print_ast(&ast);
    let start_of_file = ast.text_range();

    for descendant in ast.descendants() {
        match descendant.kind() {
            SyntaxKind::PARAMETER_LIST => {
                let param_list = descendant.to::<ParameterList>();
                let outer_scope = descendant
                    .ancestors()
                    .find(|anc| {
                        anc.is::<FnDecl>()
                            || anc.is::<Constructor>()
                            || anc.is::<Method>()
                            || anc.is::<ArrowExpr>()
                            || anc.is::<FnExpr>()
                    })
                    .unwrap();
                for param in param_list.parameters() {
                    let parameter_name = param.text();
                    let new_parameter_type = parameter_name.to_pascal_case();
                    let param_usages = gather_usages(&outer_scope, parameter_name.as_str());
                    debug!("Found param_usages: {param_usages:?} ({parameter_name})");

                    match param_usages.len() {
                        1.. => {
                            debug!("FIXER insert: {:?}", start_of_file);
                            fixer.insert_before(
                                start_of_file,
                                create_type_definition(param_usages, new_parameter_type.as_str()),
                            );
                            update_pattern(&param, &mut fixer, None, Some(new_parameter_type));
                        }
                        _ => {
                            update_pattern(&param, &mut fixer, None, None);
                        }
                    }
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
                        None => update_pattern(pattern, &mut fixer, None, None),
                        Some(Expr::Literal(literal)) if literal.is_null() => {
                            update_pattern(pattern, &mut fixer, None, None)
                        }
                        Some(Expr::NameRef(name_ref)) if name_ref.text() == "undefined" => {
                            update_pattern(pattern, &mut fixer, None, None)
                        }
                        Some(Expr::ArrayExpr(array)) if array.elements().count() == 0 => {
                            update_pattern(pattern, &mut fixer, declarator.value(), None)
                        }
                        _ => (),
                    }
                }
            }
            SyntaxKind::CATCH_CLAUSE => {
                let catch = descendant.to::<CatchClause>();
                if let Some(pattern) = catch.error() {
                    update_pattern(&pattern, &mut fixer, None, None);
                }
            }
            SyntaxKind::CLASS_DECL => {
                let class = descendant.to::<ClassDecl>();
                let props_fields = gather_usages(&ast, "props");
                let state_fields = gather_usages(&ast, "state");
                debug!("Found props: {props_fields:?}");

                match class.parent() {
                    Some(parent) if is_react_component_class(&parent) => {
                        match (
                            class.parent_type_args(),
                            props_fields.len(),
                            state_fields.len(),
                        ) {
                            (None, .., 1..) => {
                                debug!("FIXER insert: {:?}", start_of_file);
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(props_fields, "Props"),
                                );
                                debug!("FIXER insert: {:?}", start_of_file);
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(state_fields, "State"),
                                );
                                debug!("FIXER insert: {:?}", parent.range());
                                fixer.insert_after(parent.range(), "<Props, State>")
                            }
                            (None, 1.., 0) => {
                                debug!("FIXER insert: {:?}", start_of_file);
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(props_fields, "Props"),
                                );
                                debug!("FIXER insert: {:?}", parent.range());
                                fixer.insert_after(parent.range(), "<Props>")
                            }
                            (None, 0, 0) => {
                                debug!("FIXER insert: {:?}", parent.range());
                                fixer.insert_after(parent.range(), "<any, any>")
                            }
                            _ => continue,
                        };
                    }
                    _ => continue,
                }
            }
            _ => continue,
        }
    }

    fixer.apply()
}

fn create_type_definition(fields: BTreeSet<NameWithType>, name: &str) -> String {
    let mut props = String::from("");
    for field in fields {
        let resolved_type = get_surrounding_expression(field.expr).unwrap_or(String::from("any"));
        props += format!("    {}: {},\n", field.name, resolved_type).as_str();
    }

    format!(
        "
interface {name} {{
{}}}
",
        props
    )
}

fn update_pattern(
    pattern: &Pattern,
    fixer: &mut TextEditor,
    expr: Option<Expr>,
    created_type: Option<String>,
) {
    for child in pattern.syntax().children() {
        trace!("child: {child:?}");
    }

    match pattern {
        Pattern::SinglePattern(single) if single.ty().is_none() => {
            trace!("single: {single:?}");
            if let Some(span) = single.name().map(|name| name.range()) {
                if let Some(type_annotation) =
                    get_type_from_expression(expr.or(None), &created_type)
                {
                    debug!("FIXER insert: {span:?}");
                    fixer.insert_after(span, format!(": {}", type_annotation));
                }
            }
        }
        Pattern::RestPattern(_) => todo!(),
        Pattern::AssignPattern(assign) if assign.ty().is_none() => {
            // FIXME: AssignPattern.key() returns None so we work around it by querying the children instead. Should be Pattern::SinglePattern
            if let Some(type_annotation) =
                get_type_from_expression(expr.or_else(|| assign.value()), &created_type)
            {
                if let Some(name) = assign.syntax().child_with_ast::<Name>() {
                    debug!("FIXER insert: {:?}", name.range());
                    fixer.insert_after(name.range(), format!(": {}", type_annotation));
                }
            }
        }
        Pattern::ObjectPattern(obj) if obj.ty().is_none() => {
            if let Some(type_annotation) = get_type_from_expression(expr.or(None), &created_type) {
                debug!("FIXER insert: {:?}", obj.range());
                fixer.insert_after(obj.range(), format!(": {}", type_annotation));
            }
        }
        // Pattern::ArrayPattern(array) => {
        //     debug!("array pattern: {:?}", array.text());
        // }
        // Pattern::ExprPattern(_) => todo!(),
        _ => (),
    }
}

fn get_surrounding_expression(expr: Option<Expr>) -> Option<String> {
    let expr_statement = expr?
        .syntax()
        .ancestors()
        .take_while(|anc| !anc.is::<ArgList>() && !anc.is::<ParameterList>())
        .find(|anc| anc.is::<ExprStmt>())
        .map(|anc| anc.to::<ExprStmt>());

    debug!("surrounding expression: {:?}", expr_statement);

    get_type_from_expression(expr_statement?.expr(), &None)
}

fn get_type_from_expression(expr: Option<Expr>, created_type: &Option<String>) -> Option<String> {
    trace!("expr: {expr:?}");
    if created_type.is_some() {
        return created_type.clone();
    }

    match expr {
        Some(Expr::ArrayExpr(array)) => {
            let default_return = Some(String::from("any[]"));
            let mut found_type = None;
            for element in array.elements() {
                if let ExprOrSpread::Expr(expr) = element {
                    let expression_type = get_type_from_expression(Some(expr), created_type);
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
        Some(Expr::ObjectExpr(_)) | None => Some(String::from("any")),
        Some(Expr::NameRef(nr)) if nr.text() == "undefined" => Some(String::from("any")), 
        Some(Expr::AssignExpr(assign_expr)) => {
            get_type_from_expression(assign_expr.rhs(), created_type)
        }
        // FIXME: use more specific function signatures
        Some(Expr::CallExpr(call_expr)) => {
            if call_expr.callee()?.text() == "BigInt" {
                return Some(String::from("BigInt"));
            }
            Some(String::from("Function"))
        },
        _ => None

        // Expr::ArrowExpr(_) => todo!(),
        // Expr::Template(_) => todo!(),
        // Expr::ThisExpr(_) => todo!(),
        // Expr::ObjectExpr(_) => todo!(),
        // Expr::GroupingExpr(_) => todo!(),
        // Expr::BracketExpr(_) => todo!(),
        // Expr::DotExpr(_) => todo!(),
        // Expr::UnaryExpr(_) => todo!(),
        // Expr::BinExpr(_) => todo!(),
        // Expr::CondExpr(_) => todo!(),
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

fn is_react_component_class(expr: &Expr) -> bool {
    let class_names = vec!["Component", "PureComponent"];

    match expr {
        Expr::NameRef(name_ref) => class_names.contains(&name_ref.text().as_str()),
        Expr::DotExpr(dot_expr) if dot_expr.prop().is_some() => {
            class_names.contains(&dot_expr.prop().unwrap().text().as_str())
        }
        _ => false,
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
