use inflector::Inflector;
use log::{debug, trace};
use rslint_core::autofix::Fixer;
use rslint_parser::{
    ast::{
        ArrowExpr, CatchClause, ClassDecl, Constructor, Declarator, DotExpr, Expr, ExprOrSpread,
        FnDecl, FnExpr, ForStmtInit, LiteralKind, Method, Name, ObjectPattern, ParameterList,
        Pattern,
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
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(props_fields, "Props"),
                                );
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(state_fields, "State"),
                                );
                                fixer.insert_after(parent.range(), "<Props, State>")
                            }
                            (None, 1.., 0) => {
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(props_fields, "Props"),
                                );
                                fixer.insert_after(parent.range(), "<Props>")
                            }
                            (None, 0, 0) => fixer.insert_after(parent.range(), "<any, any>"),
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

fn gather_usages(root: &SyntaxNode, component_aspect: &str) -> BTreeSet<String> {
    let mut props_fields = BTreeSet::new();

    for descendant in root.descendants() {
        match descendant.kind() {
            SyntaxKind::DOT_EXPR => {
                fn expand_dot_expr(
                    expr: DotExpr,
                    props: &mut BTreeSet<String>,
                    component_aspect: &str,
                ) {
                    debug!(
                        "Found dot_expr: {expr:?} [{:?}: {:?}]",
                        expr.object(),
                        expr.prop()
                    );

                    if let Some(declarator) =
                        expr.syntax().ancestors().find(|anc| anc.is::<Declarator>())
                    {
                        debug!("Found declarator: {declarator:?}");

                        if let Some(object_pattern) = declarator
                            .descendants()
                            .find(|desc| desc.is::<ObjectPattern>())
                        {
                            let object_pattern = object_pattern.to::<ObjectPattern>();
                            debug!("Found object_pattern: {object_pattern:?}",);

                            if let Some(name) = expr.syntax().child_with_ast::<Name>() {
                                debug!("Found child name: {name} (looking for {component_aspect})");
                                if name.text() == component_aspect {
                                    for element in object_pattern.elements() {
                                        props.insert(element.text());
                                    }
                                }
                            }
                        }
                    }

                    match expr.object() {
                        Some(Expr::DotExpr(nested_dot)) => {
                            debug!("Found nested dot: {nested_dot:?}");

                            if let Some(name) = nested_dot.syntax().child_with_ast::<Name>() {
                                if name.text() == component_aspect {
                                    if let Some(name_prop) = expr.prop() {
                                        debug!("Found nested name: {name_prop:?}");
                                        props.insert(name_prop.text());
                                    }
                                }
                            }

                            expand_dot_expr(nested_dot, props, component_aspect)
                        }
                        Some(Expr::NameRef(name_ref)) => {
                            let name = name_ref.text();
                            debug!("name_ref! Found {:?}: {name}", name_ref);

                            if name == component_aspect {
                                if let Some(name_prop) = expr.prop() {
                                    debug!("Found nested name: {name_prop:?}");
                                    props.insert(name_prop.text());
                                }
                            }
                        }
                        _ => {
                            debug!("other! Found {:?}", expr.object());
                        }
                    }
                }
                let dot_expr = descendant.to::<DotExpr>();
                expand_dot_expr(dot_expr, &mut props_fields, component_aspect)
            }
            _ => (),
        }
    }

    props_fields
}

fn create_type_definition(fields: BTreeSet<String>, name: &str) -> String {
    let mut props = String::from("");
    for field in fields {
        props += format!("    {field}: any,\n").as_str();
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
    fixer: &mut Fixer,
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
                    fixer.insert_after(name.range(), format!(": {}", type_annotation));
                }
            }
        }
        Pattern::ObjectPattern(obj) if obj.ty().is_none() => {
            if let Some(type_annotation) = get_type_from_expression(expr.or(None), &created_type) {
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
