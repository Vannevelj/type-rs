use log::{debug, trace};
use rslint_parser::{
    ast::{
        ArgList, Declarator, DotExpr, Expr, ExprOrSpread, ExprStmt, LiteralKind, Name,
        ObjectPattern, ParameterList,
    },
    AstNode, SyntaxKind, SyntaxNode, SyntaxNodeExt,
};
use std::{cmp::Ordering, collections::BTreeSet};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum TypeDef {
    SimpleType(Option<Expr>),
    NestedType(BTreeSet<TypeDefinition>),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct TypeDefinition {
    pub name: String,
    pub ts_type: TypeDef,
}

impl TypeDefinition {
    fn new(name: String, expr: Option<Expr>) -> TypeDefinition {
        TypeDefinition {
            name,
            ts_type: TypeDef::SimpleType(expr),
        }
    }

    fn render(&self, depth: usize) -> String {
        let spacing = "    ".repeat(depth);
        let mut buf = String::from("");

        match &self.ts_type {
            TypeDef::SimpleType(expr) => {
                let resolved_type = get_surrounding_expression(expr).unwrap_or(String::from("any"));
                buf.push_str(format!("{spacing}{}: {},\n", self.name, resolved_type).as_str())
            }
            TypeDef::NestedType(children) => {
                buf.push_str(format!("{spacing}{}: {{", self.name).as_str());
                for child in children {
                    buf += child.render(depth + 1).as_str();
                }
                buf.push_str(format!("{spacing}}}").as_str());
            }
        }

        buf.clone()
    }
}

impl Ord for TypeDefinition {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for TypeDefinition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// WIP: change this to return a single NameWithType (rename that?)
// Build a tree of the type, going deeper when encountering a DOT_EXPR or destructuring, then render tree
pub fn define_type_based_on_usage(
    root: &SyntaxNode,
    component_aspect: &str,
) -> Option<TypeDefinition> {
    let mut root_type: Option<TypeDefinition> = None;

    for descendant in root.descendants() {
        match descendant.kind() {
            SyntaxKind::DOT_EXPR => {
                fn expand_dot_expr(
                    expr: DotExpr,
                    parent: &mut Option<TypeDefinition>,
                    component_aspect: &str,
                ) -> Option<TypeDefinition> {
                    debug!(
                        "Found dot_expr: {expr:?} [{:?}: {:?}]",
                        expr.object(),
                        expr.prop()
                    );

                    let mut child_prop: Option<TypeDefinition> = None;

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
                                        child_prop = Some(TypeDefinition::new(
                                            element.text(),
                                            expr.object(),
                                        ));
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
                                        let new_child_prop =
                                            TypeDefinition::new(name_prop.text(), expr.object());
                                        // expand_dot_expr(
                                        //     nested_dot,
                                        //     &mut child_prop,
                                        //     component_aspect,
                                        // );
                                        child_prop = Some(new_child_prop);
                                    }
                                }
                            }

                            return expand_dot_expr(nested_dot, parent, component_aspect);
                        }
                        Some(Expr::NameRef(name_ref)) => {
                            let name = name_ref.text();
                            debug!("name_ref! Found {:?}: {name}", name_ref);

                            if name == component_aspect {
                                if let Some(name_prop) = expr.prop() {
                                    debug!("Found nested name: {name_prop:?}");
                                    child_prop =
                                        Some(TypeDefinition::new(name_prop.text(), expr.object()));
                                }
                            }
                        }
                        _ => {
                            debug!("other! Found {:?}", expr.object());
                        }
                    }

                    match (parent, child_prop) {
                        (None, Some(child)) => Some(child),
                        (Some(parent), Some(child)) => {
                            let mut children = BTreeSet::new();
                            children.insert(child);
                            parent.ts_type = TypeDef::NestedType(children);
                            Some(parent.clone())
                        }
                        _ => None,
                    }
                }
                let dot_expr = descendant.to::<DotExpr>();
                root_type = expand_dot_expr(dot_expr, &mut root_type, component_aspect)
            }
            _ => (),
        }
    }

    root_type
}

fn get_surrounding_expression(expr: &Option<Expr>) -> Option<String> {
    if let Some(expr) = expr {
        let expr_statement = expr
            .syntax()
            .ancestors()
            .take_while(|anc| !anc.is::<ArgList>() && !anc.is::<ParameterList>())
            .find(|anc| anc.is::<ExprStmt>())
            .map(|anc| anc.to::<ExprStmt>());

        if let Some(expr) = expr_statement {
            let expr = expr.expr();
            debug!("surrounding expression: {:?}", expr);

            return get_type_from_expression(expr, &None);
        }
    }

    None
}

pub fn get_type_from_expression(
    expr: Option<Expr>,
    created_type: &Option<String>,
) -> Option<String> {
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

pub fn create_type_definition(def: &TypeDefinition, name: &str) -> String {
    let definition = def.render(1);

    format!(
        "
interface {name} {{
{}}}
",
        definition
    )
}
