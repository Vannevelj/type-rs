#[ctor::ctor]
fn init() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "trace"),
    );
}

mod tests {
    use type_rs::parser::add_types;

    fn compare(input: &str, expected_output: &str) {
        let output = add_types(String::from(input));
        assert_eq!(output, expected_output);
    }

    #[test]
    fn add_types_adds_component_props() {
        compare(
            "class MyComponent extends Component { }",
            "class MyComponent extends Component<any, any> { }",
        )
    }

    #[test]
    fn add_types_adds_component_props_namespace() {
        compare(
            "class MyComponent extends React.Component { }",
            "class MyComponent extends React.Component<any, any> { }",
        )
    }

    #[test]
    fn add_types_adds_component_props_pre_existing() {
        compare(
            "class MyComponent extends Component<{}> { }",
            "class MyComponent extends Component<{}> { }",
        )
    }

    #[test]
    fn add_types_adds_purecomponent_props() {
        compare(
            "class MyComponent extends PureComponent { }",
            "class MyComponent extends PureComponent<any, any> { }",
        )
    }

    #[test]
    fn add_types_adds_purecomponent_props_namespace() {
        compare(
            "class MyComponent extends React.PureComponent { }",
            "class MyComponent extends React.PureComponent<any, any> { }",
        )
    }

    #[test]
    fn add_types_different_class() {
        compare(
            "class MyComponent extends OtherType { }",
            "class MyComponent extends OtherType { }",
        )
    }

    #[test]
    fn add_types_generate_props_class_this_dot_props_single() {
        compare(
            "
class MyComponent extends Component { 
    function test() {
        console.log(this.props.wowee);
    }
}",
            "
interface Props {
    wowee: any,
}

class MyComponent extends Component<Props, any> { 
    function test() {
        console.log(this.props.wowee);
    }
}",
        )
    }

    #[test]
    fn add_types_generate_props_class_this_dot_props_multi() {
        compare(
            "
class MyComponent extends Component { 
    function test() {
        console.log(this.props.wowee);
        this.props.callback();
        this.props.callback();
    }

    render() {
        if (this.props.otherone === 5) {
            return null;
        }
    }
}",
            "
interface Props {
    callback: any,
    otherone: any,
    wowee: any,
}

class MyComponent extends Component<Props, any> { 
    function test() {
        console.log(this.props.wowee);
        this.props.callback();
        this.props.callback();
    }

    render() {
        if (this.props.otherone === 5) {
            return null;
        }
    }
}",
        )
    }
}
