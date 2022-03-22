use swc::config::{Config, JscConfig, Options};
use swc_common::errors::{ColorConfig, Handler};
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Span};
use swc_ecma_ast::{
    EsVersion, Param, Pat, TsKeywordType, TsKeywordTypeKind, TsType, TsTypeAnn, VarDeclarator,
};
use swc_ecma_parser::{Syntax, TsConfig};
use swc_ecma_transforms::pass::noop;
use swc_ecma_visit::{as_folder, Fold, VisitMut, VisitMutWith};

pub fn add_types(contents: String) -> String {
    let cm: Lrc<SourceMap> = Default::default();
    let file = cm.new_source_file(FileName::Custom("test.js".into()), String::from(contents));
    let compiler = swc::Compiler::new(cm.clone());
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let result = compiler.process_js_with_custom_pass(
        file,
        None,
        &handler,
        &Options {
            config: Config {
                jsc: JscConfig {
                    preserve_all_comments: true,
                    syntax: Some(Syntax::Typescript(TsConfig::default())),
                    target: Some(EsVersion::Es2022),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
        |_, _| noop(),
        |_, _| my_visitor(),
    );

    result.unwrap().code
}

fn my_visitor() -> impl Fold {
    as_folder(MyVisitor)
}

struct MyVisitor;
impl VisitMut for MyVisitor {
    fn visit_mut_param(&mut self, param: &mut Param) {
        param.visit_mut_children_with(self);
        update_pattern(&mut param.pat);
    }

    fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
        declarator.visit_mut_children_with(self);
        update_pattern(&mut declarator.name);
    }
}

fn update_pattern(pat: &mut Pat) {
    match pat {
        Pat::Ident(ref mut ident) => {
            ident.type_ann = Some(create_any_type());
        }
        Pat::Array(_) => todo!(),
        Pat::Rest(_) => todo!(),
        Pat::Object(_) => todo!(),
        Pat::Assign(_) => todo!(),
        Pat::Invalid(_) => todo!(),
        Pat::Expr(_) => todo!(),
    }
}

fn create_any_type() -> TsTypeAnn {
    let any_keyword = TsKeywordType {
        span: Span::default(),
        kind: TsKeywordTypeKind::TsAnyKeyword,
    };

    TsTypeAnn {
        span: Span::default(),
        type_ann: Box::new(TsType::TsKeywordType(any_keyword)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(input: &str, expected_output: &str) {
        // env_logger::init_from_env(
        //     env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "trace"),
        // );
        let output = add_types(String::from(input));
        assert_eq!(
            format!("{expected_output}\n"),
            output,
            "left is expected, right is output"
        );
    }

    #[test]
    fn add_types_function() {
        compare("function foo(a) {}", "function foo(a: any) {}");
    }

    #[test]
    fn add_types_function_default_value() {
        compare("function foo(a = 5) {}", "function foo(a: any = 5) {}");
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
// comment
function foo(
    a,
    b,
    c) 
{
    /* hello comments */
    console.log(test);
}",
            "
// comment
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
    fn add_types_array_function_default_value() {
        compare("function foo(a = []) {}", "function foo(a: any[] = []) {}");
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
            "function foo() {
    for(const key in {}) {

    }
}",
            "function foo() {
    for(const key in {}){}
}",
        );
    }

    #[test]
    fn add_types_export_default_function() {
        compare(
            "export default function foo(route) {};",
            "export default function foo(route: any) {};",
        );
    }

    #[test]
    fn add_types_class_function() {
        compare(
            "class ColorPicker {
    componentDidUpdate(prevProps, prevState) {}
}",
            "class ColorPicker {
    componentDidUpdate(prevProps: any, prevState: any) {}
}",
        );
    }

    #[test]
    fn add_types_const_arrow_function() {
        compare(
            "const mapStateToProps = (state, props) => {}",
            "const mapStateToProps = (state: any, props: any) => {}",
        );
    }

    #[test]
    fn add_types_lambda() {
        compare(
            "function foo() { sources.filter((v, k) => true ) };",
            "function foo() { sources.filter((v: any, k: any) => true ) };",
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
}
