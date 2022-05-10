use log::{debug, trace};
use rslint_parser::{
    ast::{
        ArgList, AssignExpr, CallExpr, Declarator, DotExpr, Expr, ExprOrSpread, ExprStmt,
        LiteralKind, NameRef, ObjectPattern, ObjectPatternProp, ParameterList,
    },
    AstNode, SyntaxKind, SyntaxNode, SyntaxNodeExt,
};
use std::cmp::Ordering;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum TypeDef {
    SimpleType(Option<Expr>),
    NestedType(Vec<TypeDefinition>),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct TypeDefinition {
    pub name: String,
    pub ts_type: TypeDef,
}

impl TypeDefinition {
    pub fn new(name: String, expr: Option<Expr>) -> TypeDefinition {
        trace!("Creating typedef: {name} ({expr:?})");
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
                // Deterministic ordering of the children alphabetically
                let mut sorted_children = children.clone();
                sorted_children.sort_by(|a, b| a.name.cmp(&b.name));

                if depth == 0 {
                    for child in sorted_children {
                        buf += child.render(depth + 1).as_str();
                    }
                } else {
                    buf.push_str(format!("{spacing}{}: {{\n", self.name).as_str());
                    for child in sorted_children {
                        buf += child.render(depth + 1).as_str();
                    }
                    buf.push_str(format!("{spacing}}},\n").as_str());
                }
            }
        }

        buf.clone()
    }

    pub fn add_field(&mut self, new_type_def: &mut TypeDefinition) {
        match self.ts_type {
            TypeDef::SimpleType(_) => {
                debug!("Adding field: simple type");
                self.add_child(vec![new_type_def.clone()]);
            }
            TypeDef::NestedType(_) => {
                // Does the type already have the field we want to insert? If so, merge them together
                debug!("Adding field: nested type");
                self.merge(new_type_def);
            }
        }
    }

    fn add_child(&mut self, children: Vec<TypeDefinition>) {
        match self.ts_type {
            TypeDef::SimpleType(_) => {
                let new_type = TypeDef::NestedType(children);
                self.ts_type = new_type;
            }
            TypeDef::NestedType(ref mut nested_children) => {
                for child in children {
                    // I used to use a BTreeSet to ensure this behaviour but I need a mutable iterator elsewhere,
                    // which BTreeSet does not expose. So I fell back to a vector and preventing duplicates myself
                    if !nested_children.contains(&child) {
                        nested_children.push(child)
                    }
                }
            }
        }
    }

    fn get_children(&mut self) -> Option<&mut Vec<TypeDefinition>> {
        match self.ts_type {
            TypeDef::SimpleType(_) => None,
            TypeDef::NestedType(ref mut children) => Some(children),
        }
    }

    fn merge(&mut self, other: &mut TypeDefinition) {
        let existing_definition = self
            .get_children()
            .unwrap()
            .iter_mut()
            .find(|n| n.name.eq(&other.name));

        match existing_definition {
            Some(definition) => match other.ts_type {
                TypeDef::SimpleType(_) => (),
                TypeDef::NestedType(ref mut new_nested_definitions) => {
                    definition.add_child(new_nested_definitions.clone());

                    for new_nested in new_nested_definitions {
                        definition.merge(new_nested);
                    }
                }
            },
            None => self.add_child(vec![other.clone()]),
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
        match descendant.kind() {
            SyntaxKind::DOT_EXPR => {
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

                        create_type_definition_structure(
                            &mut root_type,
                            &current_dot_expr.into(),
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
                                Some(parent) => create_type_definition_structure(
                                    &mut root_type,
                                    &parent.into(),
                                    vec![],
                                ),
                                None => include_destructured_properties(
                                    &current_dot_expr.into(),
                                    &mut root_type,
                                ),
                            }
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                }
            }
            SyntaxKind::NAME_REF => {
                let name_ref = descendant.to::<NameRef>();
                if name_ref.text() == component_aspect {
                    trace!("Found top level name_ref");

                    include_destructured_properties(&name_ref.into(), &mut root_type);
                }
            }
            _ => (),
        }
    }

    debug!("Resulting root type: {:?}", root_type);

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

fn include_destructured_properties(expr: &Expr, new_type_def: &mut TypeDefinition) {
    if let Some(Some(declarator)) = expr.syntax().parent().map(|anc| {
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

            for element in object_pattern.elements() {
                trace!("Object Pattern Element: {:?}", element.text());
                match element {
                    ObjectPatternProp::AssignPattern(_) => todo!(),
                    ObjectPatternProp::KeyValuePattern(kv) => {
                        if let Some(key) = kv.key() {
                            new_type_def.add_field(&mut TypeDefinition::new(key.text(), None));
                        }
                    }
                    ObjectPatternProp::RestPattern(_) => todo!(),
                    ObjectPatternProp::SinglePattern(single) => {
                        new_type_def.add_field(&mut TypeDefinition::new(
                            single.name().unwrap().text(),
                            None,
                        ));
                    }
                }
            }
        }
    }
}

fn create_type_definition_structure(
    parent_definition: &mut TypeDefinition,
    current_dot_expr: &DotExpr,
    mut path: Vec<String>,
) {
    let current_type_to_add_to = parent_definition;
    debug!("path: {path:?}");
    debug!("current_dot_expr: {current_dot_expr:?}");

    if let Some(name_prop) = current_dot_expr.prop() {
        debug!(
            "Found nested name: {name_prop:?} ({})",
            name_prop.syntax().text()
        );

        let mut new_type_def = TypeDefinition::new(name_prop.text(), current_dot_expr.object());

        include_destructured_properties(&current_dot_expr.clone().into(), &mut new_type_def);

        path.push(name_prop.text());

        if let Some(parent) = get_parent_dot_expr(&current_dot_expr) {
            debug!("Entering create_type_definition_structure()");
            create_type_definition_structure(&mut new_type_def, &parent, path);
        }

        /*
            If we reach the end of the dot_expr, look at the surrounding expression.
            By storing the Assign or Call expression, we can more accurately determine what type it is
        */
        if let Some(parent) = current_dot_expr.syntax().parent() {
            if parent.is::<CallExpr>() {
                debug!("found callexpr, overriding");
                let parent_expr = parent.to::<CallExpr>();
                new_type_def = TypeDefinition::new(new_type_def.name, Some(parent_expr.into()))
            }

            if parent.is::<AssignExpr>() {
                debug!("found callexpr, overriding");
                let parent_expr = parent.to::<AssignExpr>();
                new_type_def = TypeDefinition::new(new_type_def.name, Some(parent_expr.into()))
            }
        }

        current_type_to_add_to.add_field(&mut new_type_def)
    }
}

fn get_surrounding_expression(expr: &Option<Expr>) -> Option<String> {
    debug!("Fetching expression: {expr:?}");
    match expr {
        Some(Expr::AssignExpr(assign)) => {
            let expr_statement = assign
                .syntax()
                .ancestors()
                .take_while(|anc| !anc.is::<ArgList>() && !anc.is::<ParameterList>())
                .find(|anc| anc.is::<ExprStmt>())
                .map(|anc| anc.to::<ExprStmt>());

            if let Some(expr) = expr_statement {
                let expr = expr.expr();
                debug!("surrounding expression: {:?}", expr);

                return get_type_from_expression(&expr, &None);
            }

            None
        }
        _ => get_type_from_expression(expr, &None),
    }
}

pub fn get_type_from_expression(
    expr: &Option<Expr>,
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
                    let expression_type = get_type_from_expression(&Some(expr), created_type);
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
            get_type_from_expression(&assign_expr.rhs(), created_type)
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
