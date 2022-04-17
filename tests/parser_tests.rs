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
    fn add_types_function() {
        compare("function foo(a) {}", "function foo(a: any) {}");
    }

    #[test]
    fn add_types_function_default_value_number() {
        compare("function foo(a = 5) {}", "function foo(a: number = 5) {}");
    }

    #[test]
    fn add_types_function_default_value_string() {
        compare(
            "function foo(a = \"hey\") {}",
            "function foo(a: string = \"hey\") {}",
        );
    }

    #[test]
    fn add_types_function_default_value_object() {
        compare("function foo(a = {}) {}", "function foo(a: any = {}) {}");
    }

    #[test]
    fn add_types_function_default_value_array() {
        compare("function foo(a = []) {}", "function foo(a: any[] = []) {}");
    }

    #[test]
    fn add_types_function_default_value_array_string() {
        compare(
            "function foo(a = [\"s1\"]) {}",
            "function foo(a: string[] = [\"s1\"]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_array_number() {
        compare(
            "function foo(a = [1]) {}",
            "function foo(a: number[] = [1]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_array_mixed() {
        compare(
            "function foo(a = [\"s1\", 1]) {}",
            "function foo(a: any[] = [\"s1\", 1]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_array_mixed_null() {
        compare(
            "function foo(a = [1, null]) {}",
            "function foo(a: any[] = [1, null]) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_null() {
        compare(
            "function foo(a = null) {}",
            "function foo(a: any = null) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_undefined() {
        compare(
            "function foo(a = undefined) {}",
            "function foo(a: any = undefined) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_regex() {
        compare(
            "function foo(a = /.*/) {}",
            "function foo(a: RegExp = /.*/) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_bigint_suffix() {
        compare(
            "function foo(a = 9007199254740991n) {}",
            "function foo(a: BigInt = 9007199254740991n) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_bigint_ctor() {
        compare(
            "function foo(a = BigInt(9007199254740991)) {}",
            "function foo(a: BigInt = BigInt(9007199254740991)) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_bool() {
        compare(
            "function foo(a = true) {}",
            "function foo(a: boolean = true) {}",
        );
    }

    #[test]
    fn add_types_function_default_value_date() {
        compare(
            "function foo(a = new Date()) {}",
            "function foo(a = new Date()) {}",
        );
    }

    #[test]
    fn add_types_function_multi_param() {
        compare(
            "function foo(a, b, c) {}",
            "function foo(a: any, b: any, c: any) {}",
        );
    }

    #[test]
    fn add_types_respects_whitespace() {
        compare(
            "
function foo(
    a,
    b,
    c) 
{
    /* hello comments */
    console.log(test);
}",
            "
function foo(
    a: any,
    b: any,
    c: any) 
{
    /* hello comments */
    console.log(test);
}",
        );
    }

    #[test]
    fn add_types_preexisting_type() {
        compare(
            "function foo(a: string[] = []) {}",
            "function foo(a: string[] = []) {}",
        );
    }

    #[test]
    fn add_types_preserves_comments() {
        compare("// hello", "// hello");
    }

    #[test]
    fn add_types_destructured_parameter() {
        compare(
            "function getRole({ permissions, user }) { }",
            "function getRole({ permissions, user }: any) { }",
        );
    }

    #[test]
    fn add_types_for_in() {
        compare(
            "
function foo() {          
    for (const key in {}) {

    }
}",
            "
function foo() {          
    for (const key in {}) {

    }
}",
        );
    }

    #[test]
    fn add_types_export_default_function() {
        compare(
            "export default function foo(route) { }",
            "export default function foo(route: any) { }",
        );
    }

    #[test]
    fn add_types_class_function() {
        compare(
            "
class ColorPicker {
  componentDidUpdate(prevProps, prevState) { }
}",
            "
class ColorPicker {
  componentDidUpdate(prevProps: any, prevState: any) { }
}",
        );
    }

    #[test]
    fn add_types_const_arrow_function() {
        compare(
            "const mapStateToProps = (state, props) => { }",
            "const mapStateToProps = (state: any, props: any) => { }",
        );
    }

    #[test]
    fn add_types_lambda() {
        compare(
            "function foo() { sources.filter((v, k) => true; }",
            "function foo() { sources.filter((v: any, k: any) => true; }",
        );
    }

    #[test]
    fn add_types_functions_as_object_keys() {
        compare(
            "
function foo() {
  return {
    bar: (a, b) => {},
  };
};",
            "
function foo() {
  return {
    bar: (a: any, b: any) => {},
  };
};",
        );
    }

    #[test]
    fn add_types_variable_uninitialized_let() {
        compare("let test;", "let test: any;");
    }

    #[test]
    fn add_types_variable_uninitialized_const() {
        compare("const test;", "const test: any;");
    }

    #[test]
    fn add_types_variable_uninitialized_var() {
        compare("var test;", "var test: any;");
    }

    #[test]
    fn add_types_variable_uninitialized_multi() {
        compare("let test, test2;", "let test: any, test2: any;");
    }

    #[test]
    fn add_types_variable_initialized_ambiguous_null() {
        compare("let test = null;", "let test: any = null;");
    }

    #[test]
    fn add_types_variable_initialized_ambiguous_undefined() {
        compare("let test = undefined;", "let test: any = undefined;");
    }

    #[test]
    fn add_types_variable_initialized_ambiguous_array() {
        compare("let test = [];", "let test: any[] = [];");
    }

    #[test]
    fn add_types_variable_initialized_concrete_array() {
        compare("let test = [1];", "let test = [1];");
    }

    #[test]
    fn add_types_variable_initialized_variable_number_untouched() {
        compare("let test = 5;", "let test = 5;");
    }

    #[test]
    #[ignore = "rslint does not support parsing JSX"]
    fn add_types_callback_arg() {
        compare(
            "return (
<Component
    onClick={() => this.toggleSelection(entity)}
/>);",
            "return (
<Component
    onClick={() => this.toggleSelection(entity)}
/>);",
        );
    }

    #[test]
    fn add_types_try_catch() {
        compare(
            "
fn foo() {
 try {
     // thing
 } catch (err) {
     // log
 }
}",
            "
fn foo() {
 try {
     // thing
 } catch (err: any) {
     // log
 }
}",
        );
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

class MyComponent extends Component<Props> { 
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
    constructor() {
        this.state = {};
    }

    function test() {
        console.log(this.props.wowee);
        this.props.callback();
        this.props.callback();
        this.state.testNumber = 5;
    }

    render() {
        if (this.props.otherone === 5) {
            return null;
        }
    }
}",
            "
interface Props {
    callback: Function,
    otherone: any,
    wowee: any,
}

interface State {
    testNumber: number,
}

class MyComponent extends Component<Props, State> {
    constructor() {
        this.state = {};
    }

    function test() {
        console.log(this.props.wowee);
        this.props.callback();
        this.props.callback();
        this.state.testNumber = 5;
    }

    render() {
        if (this.props.otherone === 5) {
            return null;
        }
    }
}",
        )
    }

    #[test]
    fn add_types_generate_state() {
        compare(
            "
class MyComponent extends Component { 
    function test() {
        console.log(this.state.wowee);
    }
}",
            "
interface Props {
}

interface State {
    wowee: any,
}

class MyComponent extends Component<Props, State> { 
    function test() {
        console.log(this.state.wowee);
    }
}",
        )
    }

    #[test]
    fn add_types_generate_props_destructured() {
        compare(
            "
class MyComponent extends Component { 
    function test() {
        const { somefield } = this.props;
        console.log(somefield);
    }
}",
            "
interface Props {
    somefield: any,
}

class MyComponent extends Component<Props> { 
    function test() {
        const { somefield } = this.props;
        console.log(somefield);
    }
}",
        )
    }

    #[test]
    fn add_types_generate_props_destructured_multiple() {
        compare(
            "
class MyComponent extends Component { 
    function test() {
        const { somefield, otherField } = this.props;
        console.log(somefield);
    }
}",
            "
interface Props {
    otherField: any,
    somefield: any,
}

class MyComponent extends Component<Props> { 
    function test() {
        const { somefield, otherField } = this.props;
        console.log(somefield);
    }
}",
        )
    }

    #[test]
    fn add_types_generate_props_functional_component() {
        compare(
            "
function Welcome(props) {
    console.log(props.name);
    return undefined;
}",
            "
interface Props {
    name: any,
}

function Welcome(props: Props) {
    console.log(props.name);
    return undefined;
}",
        )
    }

    #[test]
    fn add_types_regular_function() {
        compare(
            "
function Welcome(args) {
    console.log(args.name);
    return undefined;
}",
            "
interface Args {
    name: any,
}

function Welcome(args: Args) {
    console.log(args.name);
    return undefined;
}",
        )
    }

    #[test]
    fn add_types_function_object_params_with_usage() {
        compare(
            "
function foo(a, b, c) {
    console.log(a);
    console.log(b.field);
}",
            "
interface B {
    field: any,
}

function foo(a: any, b: B, c: any) {
    console.log(a);
    console.log(b.field);
}",
        );
    }

    #[test]
    fn add_types_function_object_params_with_usage_multi() {
        compare(
            "
function foo(a, b, c) {
    console.log(a);
    console.log(b.field);
    console.log(b.otherfield);
}",
            "
interface B {
    field: any,
    otherfield: any,
}

function foo(a: any, b: B, c: any) {
    console.log(a);
    console.log(b.field);
    console.log(b.otherfield);
}",
        );
    }

    #[test]
    fn add_types_function_object_params_nested() {
        compare(
            "
function foo(a, b, c) {
    console.log(a);
    console.log(b.field.nested);
}",
            "
interface B {
    field: {
        nested: any,
    },
}

function foo(a: any, b: B, c: any) {
    console.log(a);
    console.log(b.field.nested);
}",
        );
    }

    #[test]
    fn add_types_params_usage_number() {
        compare(
            "
function foo(a, b, c) {
    console.log(a);
    b.field = 5;
}",
            "
interface B {
    field: number,
}

function foo(a: any, b: B, c: any) {
    console.log(a);
    b.field = 5;
}",
        );
    }

    #[test]
    fn add_types_params_usage_string() {
        compare(
            "
function foo(a, b, c) {
    console.log(a);
    b.field = \"hello\";
}",
            "
interface B {
    field: string,
}

function foo(a: any, b: B, c: any) {
    console.log(a);
    b.field = \"hello\";
}",
        );
    }

    #[test]
    fn add_types_params_usage_function() {
        compare(
            "
function foo(a, b, c) {
    console.log(a);
    b.callableField();
}",
            "
interface B {
    callableField: Function,
}

function foo(a: any, b: B, c: any) {
    console.log(a);
    b.callableField();
}",
        );
    }

    #[test]
    fn add_types_params_usage_function_in_function() {
        compare(
            "
function foo(a, b, c) {
    console.log(a);
    b.callableField(doThing(c));
}

function doThing(x) {
    return x;
}",
            "
interface B {
    callableField: Function,
}

function foo(a: any, b: B, c: any) {
    console.log(a);
    b.callableField(doThing(c));
}

function doThing(x: any) {
    return x;
}",
        );
    }
}
