use inflector::Inflector;
use log::{debug, trace};
use rslint_parser::{
    ast::{
        ArrowExpr, CatchClause, ClassDecl, Constructor, Declarator, Expr, FnDecl, FnExpr,
        ForStmtInit, Method, Name, ParameterList, Pattern,
    },
    parse_with_syntax, AstNode, Syntax, SyntaxKind, SyntaxNode, SyntaxNodeExt,
};

use crate::{
    text_editor::{TextEdit, TextEditor},
    type_definition::{
        create_type_definition, define_type_based_on_usage, get_type_from_expression, TypeDef,
        TypeDefinition,
    },
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
                    let param_usages =
                        define_type_based_on_usage(&outer_scope, parameter_name.as_str());
                    debug!("Found param_usages: {param_usages:?} ({parameter_name})");

                    match param_usages {
                        None => {
                            update_pattern(&param, &mut fixer, None, None);
                        }
                        Some(ref usages) => {
                            fixer.insert_before(
                                start_of_file,
                                create_type_definition(usages, new_parameter_type.as_str()),
                            );
                            update_pattern(&param, &mut fixer, None, Some(new_parameter_type));
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

                match class.parent() {
                    Some(parent) if is_react_component_class(&parent) => {
                        let props_fields = define_type_based_on_usage(&ast, "props");
                        let state_fields = define_type_based_on_usage(&ast, "state");
                        debug!("Found props: {props_fields:?}");

                        match (class.parent_type_args(), &props_fields, &state_fields) {
                            (None, .., Some(state_usages)) => {
                                let props_definition = props_fields.unwrap_or(TypeDefinition {
                                    name: "Props".to_string(),
                                    ts_type: TypeDef::NestedType(Vec::new()),
                                });
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(&props_definition, "Props"),
                                );
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(state_usages, "State"),
                                );
                                fixer.insert_after(parent.range(), "<Props, State>")
                            }
                            (None, Some(props_usages), None) => {
                                fixer.insert_before(
                                    start_of_file,
                                    create_type_definition(props_usages, "Props"),
                                );
                                fixer.insert_after(parent.range(), "<Props>")
                            }
                            (None, None, None) => fixer.insert_after(parent.range(), "<any, any>"),
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
            "{spaces}{name} ({:?}) {} [{:?}-{:?}]",
            &node.kind(),
            &node.text(),
            &node.text_range().start(),
            &node.text_range().end()
        );

        for child in node.children() {
            write_node(&child, depth + 1);
        }
    }

    write_node(root, 0);
}
