use log::{debug, trace};
use rslint_core::autofix::Fixer;
use rslint_parser::{
    ast::{
        CatchClause, ClassDecl, Declarator, DotExpr, Expr, ExprOrSpread, ForStmtInit, LiteralKind,
        Name, ParameterList, Pattern,
    },
    parse_with_syntax, AstNode, Syntax, SyntaxKind, SyntaxNode, SyntaxNodeExt,
};
use std::collections::BTreeSet;
use std::sync::Arc;

pub fn add_types(contents: String) -> String {
    let syntax = Syntax::default().typescript();
    let parse = parse_with_syntax(contents.as_str(), 0, syntax);
    let ast = parse.syntax();
    let arc = Arc::from(contents);
    let mut fixer = Fixer::new(arc);
    print_ast(&ast);
    let start_of_file = ast.text_range();

    for descendant in ast.descendants() {
        match descendant.kind() {
            SyntaxKind::PARAMETER_LIST => {
                let param_list = descendant.to::<ParameterList>();
                for param in param_list.parameters() {
                    update_pattern(&param, &mut fixer, None);
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
                        None => update_pattern(pattern, &mut fixer, None),
                        Some(Expr::Literal(literal)) if literal.is_null() => {
                            update_pattern(pattern, &mut fixer, None)
                        }
                        Some(Expr::NameRef(name_ref)) if name_ref.text() == "undefined" => {
                            update_pattern(pattern, &mut fixer, None)
                        }
                        Some(Expr::ArrayExpr(array)) if array.elements().count() == 0 => {
                            update_pattern(pattern, &mut fixer, declarator.value())
                        }
                        _ => (),
                    }
                }
            }
            SyntaxKind::CATCH_CLAUSE => {
                let catch = descendant.to::<CatchClause>();
                if let Some(pattern) = catch.error() {
                    update_pattern(&pattern, &mut fixer, None);
                }
            }
            SyntaxKind::CLASS_DECL => {
                let class = descendant.to::<ClassDecl>();
                let props_fields = gather_props(&ast);
                debug!("Found props: {props_fields:?}");

                match class.parent() {
                    Some(parent) if is_react_component_class(&parent) => {
                        match (class.parent_type_args(), props_fields.len()) {
                            (None, 1..) => {
                                fixer.insert_before(
                                    start_of_file,
                                    get_props_definition(props_fields),
                                );
                                fixer.insert_after(parent.range(), "<Props, any>")
                            }
                            (None, 0) => fixer.insert_after(parent.range(), "<any, any>"),
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

fn gather_props(root: &SyntaxNode) -> BTreeSet<String> {
    let mut props_fields = BTreeSet::new();

    for descendant in root.descendants() {
        match descendant.kind() {
            SyntaxKind::DOT_EXPR => {
                fn expand_dot_expr(expr: DotExpr, props: &mut BTreeSet<String>) {
                    debug!(
                        "Found dot_expr: {expr:?} [{:?}: {:?}]",
                        expr.object(),
                        expr.prop()
                    );

                    match expr.object() {
                        Some(Expr::DotExpr(nested_dot)) => {
                            if let Some(name) = nested_dot.syntax().child_with_ast::<Name>() {
                                if name.text() == "props" {
                                    if let Some(name_prop) = expr.prop() {
                                        props.insert(name_prop.text());
                                    }
                                }
                            }

                            expand_dot_expr(nested_dot, props)
                        }
                        _ => (),
                    }
                }
                let dot_expr = descendant.to::<DotExpr>();
                expand_dot_expr(dot_expr, &mut props_fields)
            }
            _ => (),
        }
    }

    props_fields
}

fn get_props_definition(fields: BTreeSet<String>) -> String {
    let mut props = String::from("");
    for field in fields {
        props += format!("    {field}: any,\n").as_str();
    }

    format!(
        "
interface Props {{
{}}}
",
        props
    )
}

fn update_pattern(pattern: &Pattern, fixer: &mut Fixer, expr: Option<Expr>) {
    for child in pattern.syntax().children() {
        trace!("child: {child:?}");
    }

    match pattern {
        Pattern::SinglePattern(single) if single.ty().is_none() => {
            trace!("single: {single:?}");
            if let Some(span) = single.name().map(|name| name.range()) {
                if let Some(type_annotation) = get_type_from_expression(expr.or(None)) {
                    fixer.insert_after(span, format!(": {}", type_annotation));
                }
            }
        }
        Pattern::RestPattern(_) => todo!(),
        Pattern::AssignPattern(assign) if assign.ty().is_none() => {
            // FIXME: AssignPattern.key() returns None so we work around it by querying the children instead. Should be Pattern::SinglePattern
            if let Some(type_annotation) = get_type_from_expression(expr.or_else(|| assign.value()))
            {
                if let Some(name) = assign.syntax().child_with_ast::<Name>() {
                    fixer.insert_after(name.range(), format!(": {}", type_annotation));
                }
            }
        }
        Pattern::ObjectPattern(obj) if obj.ty().is_none() => {
            if let Some(type_annotation) = get_type_from_expression(expr.or(None)) {
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
        Some(Expr::ObjectExpr(_)) | None => Some(String::from("any")),
        Some(Expr::NameRef(nr)) if nr.text() == "undefined" => Some(String::from("any")), 
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
