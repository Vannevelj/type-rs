#[ctor::ctor]
fn init() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "trace"),
    );
}

mod tests {
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
        root.add_field(&mut TypeDefinition::new("childprop".into(), None));

        let children = get_children(&root);
        assert_eq!(1, children.len());
    }

    #[test]
    fn type_definition_add_field_nested_child() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        root.add_field(&mut TypeDefinition {
            name: "childprop".into(),
            ts_type: TypeDef::NestedType(vec![
                TypeDefinition::new("grandchildprop".into(), None),
                TypeDefinition::new("grandchildprop2".into(), None),
            ]),
        });

        let children = get_children(&root);
        assert_eq!(1, children.len());
        assert_eq!(2, get_children(&children[0]).len());
    }

    #[test]
    fn type_definition_add_field_same_child_twice() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        root.add_field(&mut TypeDefinition::new("childprop".into(), None));

        root.add_field(&mut TypeDefinition {
            name: "childprop".into(),
            ts_type: TypeDef::SimpleType(None),
        });

        let children = get_children(&root);
        assert_eq!(1, children.len());
    }

    #[test]
    fn type_definition_add_field_same_child_with_kids() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        root.add_field(&mut TypeDefinition::new("childprop".into(), None));

        root.add_field(&mut TypeDefinition {
            name: "childprop".into(),
            ts_type: TypeDef::NestedType(vec![TypeDefinition::new("grandkiddie".into(), None)]),
        });

        let children = get_children(&root);
        assert_eq!(1, children.len());
        assert_eq!(1, get_children(&children[0]).len());
    }

    #[test]
    fn type_definition_add_field_same_child_with_kids_added_onto_child() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        let mut child = TypeDefinition::new("childprop".into(), None);

        child.add_field(&mut TypeDefinition::new("grandkiddie".into(), None));
        root.add_field(&mut child);

        let children = get_children(&root);
        assert_eq!(1, children.len());
        assert_eq!(1, get_children(&children[0]).len());
    }

    #[test]
    fn type_definition_add_field_same_child_with_grandkids() {
        let mut root = TypeDefinition::new("mytype".into(), None);
        let mut child1 = TypeDefinition::new("childprop".into(), None);
        let mut child2 = TypeDefinition::new("childprop2".into(), None);

        child1.add_field(&mut TypeDefinition::new("grandkiddie".into(), None));
        child1.add_field(&mut TypeDefinition::new("grandkiddie".into(), None)); // same as before
        child1.add_field(&mut TypeDefinition::new("grandkiddie2".into(), None));
        child1.add_field(&mut TypeDefinition::new("grandkiddie3".into(), None));
        child2.add_field(&mut TypeDefinition::new("grandkiddie".into(), None));
        root.add_field(&mut child1);
        root.add_field(&mut child2);

        let children = get_children(&root);
        assert_eq!(2, children.len());
        assert_eq!(3, get_children(&children[0]).len());
        assert_eq!(1, get_children(&children[1]).len());
    }
}
