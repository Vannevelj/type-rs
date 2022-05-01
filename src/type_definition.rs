use log::{debug, trace};
use rslint_parser::{
    ast::{
        ArgList, Declarator, DotExpr, Expr, ExprOrSpread, ExprStmt, LiteralKind, Name,
        ObjectPattern, ParameterList, SinglePattern, ThisExpr,
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
        trace!("Creating typedef: {name}");
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
                if depth == 0 {
                    for child in children {
                        buf += child.render(depth + 1).as_str();
                    }
                } else {
                    buf.push_str(format!("{spacing}{}: {{\n", self.name).as_str());
                    for child in children {
                        buf += child.render(depth + 1).as_str();
                    }
                    buf.push_str(format!("{spacing}}},\n").as_str());
                }
            }
        }

        buf.clone()
    }

    fn add_field(&mut self, new_type_def: TypeDefinition) {
        match self.ts_type {
            TypeDef::SimpleType(_) => {
                let mut children = BTreeSet::new();
                children.insert(new_type_def);
                let new_type = TypeDef::NestedType(children);
                self.ts_type = new_type;
            }
            TypeDef::NestedType(ref mut nested_type) => {
                nested_type.insert(new_type_def);
            }
        }
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

/**
    Dot expr (DOT_EXPR) a.sa.nestedlongname [23-42]
        Dot expr (DOT_EXPR) a.sa [23-27]
            Name ref (NAME_REF) a [23-24]
            Name (NAME) sa [25-27]
        Name (NAME) nestedlongname [28-42]
*/
pub fn define_type_based_on_usage(
    root: &SyntaxNode,
    component_aspect: &str,
) -> Option<TypeDefinition> {
    let mut root_type = TypeDefinition {
        name: component_aspect.to_string(),
        ts_type: TypeDef::SimpleType(None),
    };

    for descendant in root.descendants() {
        if let SyntaxKind::DOT_EXPR = descendant.kind() {
            let current_dot_expr = {
                let mut inner = descendant.to::<DotExpr>();
                while let Some(child) = inner.syntax().child_with_kind(SyntaxKind::DOT_EXPR) {
                    inner = child.to::<DotExpr>();
                }
                inner
            };

            /* Take the inner-most DotExpr if we are accessing a.b.c.
             Take the second to inner-most DotExpr if we are accessing this.props.a.b.c
             This makes sure that we don't create an interface of the structure
             ```
                interface Props {
                    props: {
                        a
                    }
                }
             ```
            */
            match current_dot_expr.object() {
                /*  Used in
                ```
                    function(a) {
                        console.log(a.b.c.d)
                    }
                ```
                */
                Some(Expr::NameRef(name_ref)) if name_ref.text() == component_aspect => {
                    debug!("name_ref! Found {:?}", name_ref);

                    root_type = create_type_definition_structure(
                        root_type.clone(),
                        current_dot_expr,
                        vec![],
                    )
                }
                /*  Used in
                ```
                    class Component {
                        console.log(this.props.a)
                    }
                ```
                */
                Some(Expr::ThisExpr(this_expr)) => {
                    let corresponding_name = this_expr.syntax().next_sibling().unwrap().text();
                    if corresponding_name == component_aspect {
                        debug!("this_expr! Found {:?}", this_expr);

                        match get_parent_dot_expr(&current_dot_expr) {
                            Some(parent) => {
                                root_type = create_type_definition_structure(
                                    root_type.clone(),
                                    parent,
                                    vec![],
                                )
                            }
                            None => {
                                include_destructured_properties(&current_dot_expr, &mut root_type)
                            }
                        }
                    } else {
                        continue;
                    }
                }
                _ => continue,
            }
        }
    }

    // Don't create an interface definition if there are no nested usages
    match root_type.ts_type {
        TypeDef::SimpleType(_) => None,
        TypeDef::NestedType(_) => Some(root_type),
    }
}

fn get_parent_dot_expr(expr: &DotExpr) -> Option<DotExpr> {
    let parent = expr.syntax().parent();
    if let Some(parent) = parent {
        if parent.is::<DotExpr>() {
            return Some(parent.to::<DotExpr>());
        }
    }

    None
}

fn include_destructured_properties(current_dot_expr: &DotExpr, new_type_def: &mut TypeDefinition) {
    if let Some(Some(declarator)) = current_dot_expr.syntax().parent().map(|anc| {
        if anc.is::<Declarator>() {
            Some(anc.to::<Declarator>())
        } else {
            None
        }
    }) {
        debug!("Found declarator: {declarator:?}");

        if let Some(object_pattern) = declarator
            .syntax()
            .child_with_kind(SyntaxKind::OBJECT_PATTERN)
        {
            let object_pattern = object_pattern.to::<ObjectPattern>();
            debug!("Found object_pattern: {object_pattern:?}");

            for descendant in object_pattern.syntax().descendants() {
                if descendant.is::<SinglePattern>() {
                    let pattern = descendant.to::<SinglePattern>();
                    new_type_def.add_field(TypeDefinition::new(
                        pattern.name().unwrap().text(),
                        current_dot_expr.object(),
                    ));
                }
            }
        }
    }
}

fn create_type_definition_structure(
    parent_definition: TypeDefinition,
    current_dot_expr: DotExpr,
    mut path: Vec<String>,
) -> TypeDefinition {
    let mut current_type_to_add_to = parent_definition;
    debug!("path: {path:?}");

    if let Some(name_prop) = current_dot_expr.prop() {
        debug!(
            "Found nested name: {name_prop:?} ({})",
            name_prop.syntax().text()
        );

        let mut new_type_def = TypeDefinition::new(name_prop.text(), current_dot_expr.object());

        include_destructured_properties(&current_dot_expr, &mut new_type_def);

        path.push(name_prop.text());

        match get_parent_dot_expr(&current_dot_expr) {
            Some(parent) => {
                debug!("Entering create_type_definition_structure()");
                let type_with_children =
                    create_type_definition_structure(new_type_def, parent, path);

                current_type_to_add_to.add_field(type_with_children);
            }
            None => current_type_to_add_to.add_field(new_type_def),
        }
    }

    debug!("Returning current_type");
    current_type_to_add_to
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
    debug!("Type definition: {def:?}");
    let definition = def.render(0);

    format!(
        "
interface {name} {{
{}}}
",
        definition
    )
}
