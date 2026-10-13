use slint_build::CompilerConfiguration;

fn main() {
    let conf = CompilerConfiguration::default().with_style("fluent-dark".to_string());
    slint_build::compile_with_config("ui/ui.slint", conf).unwrap();
}
