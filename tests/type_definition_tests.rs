#[ctor::ctor]
fn init() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "trace"),
    );
}

mod tests {
    use std::collections::BTreeSet;

    use pretty_assertions::assert_eq;
    use type_rs::type_definition::{TypeDef, TypeDefinition};

    fn get_children(definition: &TypeDefinition) -> Vec<TypeDefinition> {
        match definition.ts_type.clone() {
            TypeDef::SimpleType(_) => unreachable!(),
            TypeDef::NestedType(children) => children.into_iter().collect(),
        }
    }

    #[test]
    fn type_definition_add_field_simple_child() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        root.add_field(TypeDefinition::new("childprop".into(), None));

        let children = get_children(&root);
        assert_eq!(1, children.len());
    }

    #[test]
    fn type_definition_add_field_nested_child() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        let mut children = BTreeSet::new();
        children.insert(TypeDefinition::new("grandchildprop".into(), None));
        children.insert(TypeDefinition::new("grandchildprop2".into(), None));
        root.add_field(TypeDefinition {
            name: "childprop".into(),
            ts_type: TypeDef::NestedType(children),
        });

        let children = get_children(&root);
        assert_eq!(1, children.len());
        assert_eq!(2, get_children(&children[0]).len());
    }

    #[test]
    fn type_definition_add_field_same_child_twice() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        root.add_field(TypeDefinition::new("childprop".into(), None));

        root.add_field(TypeDefinition {
            name: "childprop".into(),
            ts_type: TypeDef::SimpleType(None),
        });

        let children = get_children(&root);
        assert_eq!(1, children.len());
    }

    #[test]
    fn type_definition_add_field_same_child_with_kids() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        root.add_field(TypeDefinition::new("childprop".into(), None));

        let mut kids = BTreeSet::new();
        kids.insert(TypeDefinition::new("grandkiddie".into(), None));
        root.add_field(TypeDefinition {
            name: "childprop".into(),
            ts_type: TypeDef::NestedType(kids),
        });

        let children = get_children(&root);
        assert_eq!(1, children.len());
        assert_eq!(1, get_children(&children[0]).len());
    }
}
