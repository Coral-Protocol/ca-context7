use clap::Parser;
use coral_rs::agent::Agent;
use coral_rs::agent_loop::AgentLoop;
use coral_rs::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use coral_rs::mcp_server::McpConnectionBuilder;
use coral_rs::repeating_prompt_stream::repeating_prompt_stream;
use coral_rs::rig::client::{CompletionClient, ProviderClient};
use coral_rs::rig::providers::openai::GPT_4_1;
use coral_rs::rig::providers::openrouter;
use coral_rs::rmcp::model::ProtocolVersion;
use coral_rs::telemetry::TelemetryMode;

#[derive(Parser, Debug)]
struct Config {
    #[clap(long, env = "LIBRARY_ID")]
    library_id: String,

    #[clap(long, env = "SYSTEM_PROMPT_SUFFIX")]
    prompt_suffix: Option<String>,

    #[clap(long, env = "LOOP_PROMPT_SUFFIX")]
    loop_prompt_suffix: Option<String>,

    #[clap(long, env = "TEMPERATURE")]
    temperature: f64,

    #[clap(long, env = "MAX_TOKENS")]
    max_tokens: u64,

    #[clap(long)]
    #[clap(long, env = "ENABLE_TELEMETRY")]
    enable_telemetry: bool,

    #[clap(long)]
    #[clap(long, env = "LOOP_DELAY")]
    loop_delay: Option<humantime::Duration>,

    #[clap(long)]
    #[clap(long, env = "LOOP_MAX_REPS")]
    loop_max_reps: usize,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let model = GPT_4_1;
    let config = Config::parse();

    let coral = McpConnectionBuilder::from_coral_env()
        .connect()
        .await.expect("Failed to connect to the Coral server");

    let context7 = McpConnectionBuilder::stdio(
        "/app/run.sh",
        vec![],
        "context7"
    )
        .protocol_version(ProtocolVersion::V_2024_11_05)
        .connect()
        .await.expect("Failed to connect to the context7 MCP server");

    let completion_agent = openrouter::Client::from_env()
        .agent(model)
        .temperature(config.temperature)
        .max_tokens(config.max_tokens)
        .build();

    let mut preamble = coral.prompt_with_resources();
    if let Some(prompt_suffix) = config.prompt_suffix {
        preamble = preamble.string(prompt_suffix);
    }

    let mut agent = Agent::new(completion_agent)
        .preamble(preamble)
        .mcp_server(coral)
        .mcp_server(context7);

    if config.enable_telemetry {
        agent = agent.telemetry(TelemetryMode::OpenAI, model);
    }

    let mut evaluating_prompt = CompletionEvaluatedPrompt::new()
        .string(format!("1. If you haven't already, call the get-library-docs tool with context7CompatibleLibraryID = {}", config.library_id))
        .string("2. Repeatedly call coral_wait_for_mentions tool until it returns messages")
        .string("3. Analyse the messages returned, make note of the request and thread ID")
        .string("4. Using the returned documentation, quoting where possible, respond (using the coral_send_message tool) to any questions returned by the coral_wait_for_mentions tool");

    if let Some(loop_prompt_suffix) = config.loop_prompt_suffix {
        evaluating_prompt = evaluating_prompt.string(loop_prompt_suffix);
    }

    let prompt_stream = repeating_prompt_stream(
        evaluating_prompt,
        config.loop_delay.map(Into::into),
        config.loop_max_reps
    );

    AgentLoop::new(agent, prompt_stream)
        .iteration_tool_quota(Some(4096))
        .execute()
        .await
        .expect("Agent loop failed");
}
