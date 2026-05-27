#[path = "common/harness_record.rs"]
mod harness_record;

use harness_record::HarnessRun;

fn main() -> Result<(), serde_json::Error> {
    let run = HarnessRun {
        schema_version: HarnessRun::SCHEMA_VERSION,
        producer: "rig-mcp/examples/harness_record".to_string(),
        task: "Expose a tool-loop run over an MCP transport.".to_string(),
        first_model_output: "MCP bridge selected loopback transport.".to_string(),
        invocations: Vec::new(),
        dispatch_results: Vec::new(),
        final_answer: "Harness record shape is shared with rig-compose.".to_string(),
        passed_assertions: vec!["schema-shared".to_string()],
    };

    println!("{}", serde_json::to_string_pretty(&run)?);
    Ok(())
}
