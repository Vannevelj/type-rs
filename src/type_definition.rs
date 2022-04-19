use log::debug;
use rslint_parser::{
    ast::{Declarator, DotExpr, Expr, Name, ObjectPattern},
    AstNode, SyntaxKind, SyntaxNode, SyntaxNodeExt,
};
use std::{cmp::Ordering, collections::BTreeSet};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct NameWithType {
    pub name: String,
    pub expr: Option<Expr>,
}

impl Ord for NameWithType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for NameWithType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// WIP: change this to return a single NameWithType (rename that?)
// Build a tree of the type, going deeper when encountering a DOT_EXPR or destructuring, then render tree
pub fn gather_usages(root: &SyntaxNode, component_aspect: &str) -> BTreeSet<NameWithType> {
    let mut props_fields = BTreeSet::new();

    for descendant in root.descendants() {
        match descendant.kind() {
            SyntaxKind::DOT_EXPR => {
                fn expand_dot_expr(
                    expr: DotExpr,
                    props: &mut BTreeSet<NameWithType>,
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
                                        props.insert(NameWithType {
                                            name: element.text(),
                                            expr: expr.object(),
                                        });
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
                                        props.insert(NameWithType {
                                            name: name_prop.text(),
                                            expr: expr.object(),
                                        });
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
                                    props.insert(NameWithType {
                                        name: name_prop.text(),
                                        expr: expr.object(),
                                    });
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
