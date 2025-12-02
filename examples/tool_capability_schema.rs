use fluent_agent::tools::ToolCapabilityConfig;

fn main() {
    println!("Tool Capability Configuration JSON Schema:");
    println!("{}", ToolCapabilityConfig::json_schema());
}
