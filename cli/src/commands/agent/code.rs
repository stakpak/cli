use crate::config::AppConfig;
use crate::utils::local_context::LocalContext;
use futures_util::{Stream, StreamExt};
use rmcp::model::{CallToolRequestParam, CallToolResult};
use stakpak_api::{Client, ClientConfig};
use stakpak_mcp_client::ClientManager;
use stakpak_mcp_server::MCPServerConfig;
use stakpak_shared::models::integrations::openai::{
    ChatCompletionChoice, ChatCompletionResponse, ChatCompletionStreamResponse, ChatMessage,
    FinishReason, FunctionCall, FunctionCallDelta, FunctionDefinition, MessageContent, Role, Tool,
    ToolCall, ToolCallResult, Usage,
};
use stakpak_tui::{InputEvent, OutputEvent};
use uuid::Uuid;

// Helper to convert tools_map to Vec<Tool>
fn convert_tools_map(
    tools_map: &std::collections::HashMap<String, Vec<rmcp::model::Tool>>,
) -> Vec<Tool> {
    tools_map
        .iter()
        .flat_map(|(_name, tools)| {
            tools.iter().map(|tool| Tool {
                r#type: "function".to_string(),
                function: FunctionDefinition {
                    name: tool.name.clone().into_owned(),
                    description: tool.description.clone().map(|d| d.to_string()),
                    parameters: serde_json::Value::Object((*tool.input_schema).clone()),
                },
            })
        })
        .collect()
}

// Helper to create a ChatMessage from user input
fn user_message(user_input: String) -> ChatMessage {
    ChatMessage {
        role: Role::User,
        content: Some(MessageContent::String(user_input)),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

fn tool_result(tool_call_id: String, result: String) -> ChatMessage {
    ChatMessage {
        role: Role::Tool,
        content: Some(MessageContent::String(result)),
        name: None,
        tool_calls: None,
        tool_call_id: Some(tool_call_id),
    }
}

// Helper to send a message to the TUI
async fn send_input_event(
    input_tx: &tokio::sync::mpsc::Sender<InputEvent>,
    event: InputEvent,
) -> Result<(), String> {
    input_tx.send(event).await.map_err(|e| e.to_string())
}

// Helper to send tool call messages to the TUI
async fn send_tool_call(
    input_tx: &tokio::sync::mpsc::Sender<InputEvent>,
    tool_call: &stakpak_shared::models::integrations::openai::ToolCall,
) -> Result<(), String> {
    send_input_event(input_tx, InputEvent::RunToolCall(tool_call.clone())).await?;
    Ok(())
}

async fn run_tool_call(
    client_manager: &ClientManager,
    tools_map: &std::collections::HashMap<String, Vec<rmcp::model::Tool>>,
    tool_call: &stakpak_shared::models::integrations::openai::ToolCall,
) -> Result<Option<CallToolResult>, String> {
    let tool_name = &tool_call.function.name;
    let client_name = tools_map
        .iter()
        .find(|(_, tools)| tools.iter().any(|tool| tool.name == *tool_name))
        .map(|(name, _)| name.clone());

    if let Some(client_name) = client_name {
        let client = client_manager
            .get_client(&client_name)
            .await
            .map_err(|e| e.to_string())?;
        let result = client
            .call_tool(CallToolRequestParam {
                name: tool_name.clone().into(),
                arguments: Some(
                    serde_json::from_str(&tool_call.function.arguments)
                        .map_err(|e| e.to_string())?,
                ),
            })
            .await
            .map_err(|e| e.to_string())?;

        return Ok(Some(result));
    }

    Ok(None)
}

pub async fn get_checkpoint_messages(
    client: &Client,
    checkpoint_id: &String,
) -> Result<Vec<ChatMessage>, String> {
    let checkpoint_uuid = Uuid::parse_str(checkpoint_id).map_err(|_| {
        format!(
            "Invalid checkpoint ID '{}' - must be a valid UUID",
            checkpoint_id
        )
    })?;

    let checkpoint = client
        .get_agent_checkpoint(checkpoint_uuid)
        .await
        .map_err(|e| e.to_string())?;
    let checkpoint_output: stakpak_api::models::AgentOutput = checkpoint.output;

    if let stakpak_api::models::AgentOutput::PabloV1 { messages, .. } = checkpoint_output {
        return Ok(messages.clone());
    }

    Ok(vec![])
}

pub async fn process_responses_stream(
    stream: impl Stream<Item = Result<Vec<ChatCompletionStreamResponse>, String>>,
    input_tx: &tokio::sync::mpsc::Sender<InputEvent>,
) -> Result<ChatCompletionResponse, String> {
    let mut stream = Box::pin(stream);

    let mut chat_completion_response = ChatCompletionResponse {
        id: "".to_string(),
        object: "".to_string(),
        created: 0,
        model: "".to_string(),
        choices: vec![],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
        system_fingerprint: None,
    };

    let mut chat_message = ChatMessage {
        role: Role::Assistant,
        content: None,
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };
    let message_id = Uuid::new_v4();

    while let Some(response) = stream.next().await {
        send_input_event(input_tx, InputEvent::Loading(false)).await?;
        if let Ok(responses) = response {
            for response in responses {
                let delta = &response.choices[0].delta;

                chat_completion_response = ChatCompletionResponse {
                    id: response.id.clone(),
                    object: response.object.clone(),
                    created: response.created,
                    model: response.model.clone(),
                    choices: vec![],
                    usage: Usage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    },
                    system_fingerprint: None,
                };

                if let Some(content) = &delta.content {
                    chat_message.content =
                        Some(MessageContent::String(match chat_message.content {
                            Some(MessageContent::String(old_content)) => old_content + content,
                            _ => content.clone(),
                        }));
                    for letter in content.chars() {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                        send_input_event(
                            input_tx,
                            InputEvent::StreamAssistantMessage(message_id, letter.to_string()),
                        )
                        .await?;
                    }
                }

                if let Some(tool_calls) = &delta.tool_calls {
                    for delta_tool_call in tool_calls {
                        if chat_message.tool_calls.is_none() {
                            chat_message.tool_calls = Some(vec![]);
                        }

                        let tool_calls_vec = chat_message.tool_calls.as_mut().unwrap();

                        match tool_calls_vec.get_mut(delta_tool_call.index) {
                            Some(tool_call) => {
                                let delta_func = delta_tool_call.function.as_ref().unwrap_or(
                                    &FunctionCallDelta {
                                        name: None,
                                        arguments: None,
                                    },
                                );
                                tool_call.function.arguments = tool_call.function.arguments.clone()
                                    + delta_func.arguments.as_deref().unwrap_or("");
                            }
                            None => {
                                // push empty tool calls until the index is reached
                                tool_calls_vec.extend(
                                    (tool_calls_vec.len()..delta_tool_call.index).map(|_| {
                                        ToolCall {
                                            id: "".to_string(),
                                            r#type: "function".to_string(),
                                            function: FunctionCall {
                                                name: "".to_string(),
                                                arguments: "".to_string(),
                                            },
                                        }
                                    }),
                                );

                                tool_calls_vec.push(ToolCall {
                                    id: delta_tool_call.id.clone().unwrap_or_default(),
                                    r#type: "function".to_string(),
                                    function: FunctionCall {
                                        name: delta_tool_call
                                            .function
                                            .as_ref()
                                            .unwrap_or(&FunctionCallDelta {
                                                name: None,
                                                arguments: None,
                                            })
                                            .name
                                            .as_deref()
                                            .unwrap_or("")
                                            .to_string(),
                                        arguments: "".to_string(),
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // filter out empty tool calls
    chat_message.tool_calls = Some(
        chat_message
            .tool_calls
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .filter(|tool_call| !tool_call.id.is_empty())
            .cloned()
            .collect::<Vec<ToolCall>>(),
    );

    chat_completion_response.choices.push(ChatCompletionChoice {
        index: 0,
        message: chat_message.clone(),
        finish_reason: FinishReason::Stop,
        logprobs: None,
    });

    Ok(chat_completion_response)
}

pub struct RunInteractiveConfig {
    pub checkpoint_id: Option<String>,
    pub local_context: Option<LocalContext>,
}

pub async fn run(ctx: AppConfig, config: RunInteractiveConfig) -> Result<(), String> {
    let mut messages: Vec<ChatMessage> = Vec::new();
    let mut tools_queue: Vec<ToolCall> = Vec::new();
    let (input_tx, input_rx) = tokio::sync::mpsc::channel::<InputEvent>(100);
    let (output_tx, mut output_rx) = tokio::sync::mpsc::channel::<OutputEvent>(100);

    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let _ = stakpak_mcp_server::start_server(MCPServerConfig {
            api: ClientConfig {
                api_key: ctx_clone.api_key,
                api_endpoint: ctx_clone.api_endpoint,
            },
        })
        .await;
    });

    // Initialize clients and tools
    let clients = ClientManager::new().await.map_err(|e| e.to_string())?;
    let tools_map = clients.get_tools().await.map_err(|e| e.to_string())?;
    let tools = convert_tools_map(&tools_map);

    // Spawn TUI task
    let tui_handle = tokio::spawn(async move {
        let _ = stakpak_tui::run_tui(input_rx, output_tx)
            .await
            .map_err(|e| e.to_string());
    });

    // Spawn client task
    let client_handle: tokio::task::JoinHandle<Result<(), String>> = tokio::spawn(async move {
        let client = Client::new(&ClientConfig {
            api_key: ctx.api_key,
            api_endpoint: ctx.api_endpoint,
        })
        .map_err(|e| e.to_string())?;

        let data = match client.get_my_account().await {
            Ok(d) => d,
            Err(e) => {
                let _ = input_tx
                    .send(InputEvent::Error(format!("Account error: {e}")))
                    .await;
                return Ok(());
            }
        };
        send_input_event(&input_tx, InputEvent::GetStatus(data.to_text())).await?;

        if let Some(checkpoint_id) = config.checkpoint_id {
            let mut checkpoint_messages =
                match get_checkpoint_messages(&client, &checkpoint_id).await {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = input_tx
                            .send(InputEvent::Error(format!("Checkpoint error: {e}")))
                            .await;
                        return Err(e.to_string());
                    }
                };
            // Append checkpoint_id to the last assistant message if present
            if let Some(last_message) = checkpoint_messages
                .iter_mut()
                .rev()
                .find(|message| message.role != Role::User && message.role != Role::Tool)
            {
                if last_message.role == Role::Assistant {
                    last_message.content = Some(MessageContent::String(format!(
                        "{}\n<checkpoint_id>{}</checkpoint_id>",
                        last_message
                            .content
                            .as_ref()
                            .unwrap_or(&MessageContent::String(String::new())),
                        checkpoint_id
                    )));
                }
            }

            for message in &checkpoint_messages {
                match message.role {
                    Role::Assistant | Role::User => {
                        if let Some(content) = &message.content {
                            let _ = input_tx
                                .send(InputEvent::InputSubmittedWith(content.to_string()))
                                .await;
                        }
                    }
                    Role::Tool => {
                        let tool_call = checkpoint_messages
                            .iter()
                            .find(|checkpoint_message| {
                                checkpoint_message
                                    .tool_calls
                                    .as_ref()
                                    .is_some_and(|tool_calls| {
                                        message.tool_call_id.as_ref().is_some_and(|tool_call_id| {
                                            tool_calls
                                                .iter()
                                                .any(|tool_call| tool_call.id == *tool_call_id)
                                        })
                                    })
                            })
                            .and_then(|chat_message| {
                                chat_message.tool_calls.as_ref().and_then(|tool_calls| {
                                    message.tool_call_id.as_ref().and_then(|tool_call_id| {
                                        tool_calls
                                            .iter()
                                            .find(|tool_call| tool_call.id == *tool_call_id)
                                    })
                                })
                            });

                        if let Some(tool_call) = tool_call {
                            let _ = send_input_event(
                                &input_tx,
                                InputEvent::ToolResult(ToolCallResult {
                                    call: tool_call.clone(),
                                    result: message
                                        .content
                                        .as_ref()
                                        .unwrap_or(&MessageContent::String(String::new()))
                                        .to_string(),
                                }),
                            )
                            .await;
                        }
                    }
                    _ => {}
                }
            }

            let tool_calls = checkpoint_messages
                .last()
                .filter(|msg| msg.role == Role::Assistant)
                .and_then(|msg| msg.tool_calls.as_ref());

            if let Some(tool_calls) = tool_calls {
                tools_queue.extend(tool_calls.clone());
                if !tools_queue.is_empty() {
                    let initial_tool_call = tools_queue.remove(0);
                    send_tool_call(&input_tx, &initial_tool_call).await?;
                }
            }

            messages.extend(checkpoint_messages);
        }

        while let Some(output_event) = output_rx.recv().await {
            match output_event {
                OutputEvent::UserMessage(user_input) => {
                    let (user_input, local_context) =
                        add_local_context(&messages, &user_input, &config.local_context);
                    if let Some(local_context) = local_context {
                        send_input_event(
                            &input_tx,
                            InputEvent::InputSubmittedWith(local_context.to_string()),
                        )
                        .await?;
                    }
                    messages.push(user_message(user_input));
                }
                OutputEvent::AcceptTool(tool_call) => {
                    send_input_event(&input_tx, InputEvent::Loading(true)).await?;
                    let result = run_tool_call(&clients, &tools_map, &tool_call).await?;
                    send_input_event(&input_tx, InputEvent::Loading(false)).await?;
                    if let Some(result) = result {
                        let result_content = result
                            .content
                            .iter()
                            .map(|c| match c.raw.as_text() {
                                Some(text) => text.text.clone(),
                                None => String::new(),
                            })
                            .collect::<Vec<String>>()
                            .join("\n");

                        messages.push(tool_result(tool_call.clone().id, result_content.clone()));

                        send_input_event(
                            &input_tx,
                            InputEvent::ToolResult(ToolCallResult {
                                call: tool_call.clone(),
                                result: result_content,
                            }),
                        )
                        .await?;
                    }

                    if !tools_queue.is_empty() {
                        let tool_call = tools_queue.remove(0);
                        send_tool_call(&input_tx, &tool_call).await?;
                        continue;
                    }
                }
                OutputEvent::RejectTool(_tool_call) => {
                    if !tools_queue.is_empty() {
                        let tool_call = tools_queue.remove(0);
                        send_tool_call(&input_tx, &tool_call).await?;
                        continue;
                    }
                }
            }
            send_input_event(&input_tx, InputEvent::Loading(true)).await?;

            let mut stream = match client
                .chat_completion_stream(messages.clone(), Some(tools.clone()))
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    send_input_event(&input_tx, InputEvent::Error(format!("Stream error: {e}")))
                        .await?;
                    return Err(e.to_string());
                }
            };

            let response = match process_responses_stream(&mut stream, &input_tx).await {
                Ok(response) => response,
                Err(e) => {
                    send_input_event(&input_tx, InputEvent::Loading(false)).await?;
                    send_input_event(
                        &input_tx,
                        InputEvent::Error(format!("Response stream error: {e}")),
                    )
                    .await?;
                    return Err(e.to_string());
                }
            };
            send_input_event(&input_tx, InputEvent::Loading(false)).await?;

            messages.push(response.choices[0].message.clone());

            // Send tool calls to TUI if present
            if let Some(tool_calls) = &response.choices[0].message.tool_calls {
                tools_queue.extend(tool_calls.clone());
                if !tools_queue.is_empty() {
                    let tool_call = tools_queue.remove(0);
                    send_tool_call(&input_tx, &tool_call).await?;
                    continue;
                }
            }
        }
        Ok(())
    });

    // Wait for both tasks to finish
    let (_, client_res) = tokio::try_join!(tui_handle, client_handle).map_err(|e| e.to_string())?;
    client_res?;
    Ok(())
}

pub struct RunNonInteractiveConfig {
    pub prompt: String,
    pub approve: bool,
    pub verbose: bool,
    pub checkpoint_id: Option<String>,
    pub local_context: Option<LocalContext>,
}

pub async fn run_non_interactive(
    ctx: AppConfig,
    config: RunNonInteractiveConfig,
) -> Result<(), String> {
    let mut chat_messages: Vec<ChatMessage> = Vec::new();

    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let _ = stakpak_mcp_server::start_server(MCPServerConfig {
            api: ClientConfig {
                api_key: ctx_clone.api_key,
                api_endpoint: ctx_clone.api_endpoint,
            },
        })
        .await;
    });

    let clients = ClientManager::new().await.map_err(|e| e.to_string())?;
    let tools_map = clients.get_tools().await.map_err(|e| e.to_string())?;
    let tools = convert_tools_map(&tools_map);

    let client = Client::new(&ClientConfig {
        api_key: ctx.api_key,
        api_endpoint: ctx.api_endpoint,
    })
    .map_err(|e| e.to_string())?;

    if let Some(checkpoint_id) = config.checkpoint_id {
        let mut checkpoint_messages = get_checkpoint_messages(&client, &checkpoint_id).await?;

        // Append checkpoint_id to the last assistant message if present
        if let Some(last_message) = checkpoint_messages
            .iter_mut()
            .rev()
            .find(|message| message.role != Role::User && message.role != Role::Tool)
        {
            if last_message.role == Role::Assistant {
                last_message.content = Some(MessageContent::String(format!(
                    "{}\n<checkpoint_id>{}</checkpoint_id>",
                    last_message
                        .content
                        .as_ref()
                        .unwrap_or(&MessageContent::String(String::new())),
                    checkpoint_id
                )));
            }
        }
        chat_messages.extend(checkpoint_messages);
    }

    if let Some(message) = chat_messages.last() {
        if config.approve && message.tool_calls.is_some() {
            // Clone the tool_calls to avoid borrowing message while mutating chat_messages
            let tool_calls = message.tool_calls.as_ref().unwrap().clone();
            for tool_call in tool_calls.iter() {
                let result = run_tool_call(&clients, &tools_map, tool_call).await?;
                if let Some(result) = result {
                    if !config.verbose {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    }

                    let result_content = result
                        .content
                        .iter()
                        .map(|c| match c.raw.as_text() {
                            Some(text) => text.text.clone(),
                            None => String::new(),
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    chat_messages.push(tool_result(tool_call.id.clone(), result_content.clone()));
                }
            }
        }
    }

    if !config.prompt.is_empty() {
        let (user_input, _local_context) =
            add_local_context(&chat_messages, &config.prompt, &config.local_context);
        chat_messages.push(user_message(user_input));
    }

    let response = client
        .chat_completion(chat_messages.clone(), Some(tools))
        .await
        .map_err(|e| e.to_string())?;

    chat_messages.push(response.choices[0].message.clone());

    match config.verbose {
        true => {
            println!("{}", serde_json::to_string_pretty(&chat_messages).unwrap());
        }
        false => {
            println!(
                "{}",
                serde_json::to_string_pretty(&response.choices[0].message).unwrap()
            );
        }
    }

    Ok(())
}

fn add_local_context<'a>(
    messages: &'a [ChatMessage],
    user_input: &'a str,
    local_context: &'a Option<LocalContext>,
) -> (String, Option<&'a LocalContext>) {
    if let Some(local_context) = local_context {
        // only add local context if this is the first message
        if messages.is_empty() {
            let formatted_input = format!(
                "{}\n\n<local_context>\n{}\n</local_context>",
                user_input, local_context
            );
            (formatted_input, Some(local_context))
        } else {
            (user_input.to_string(), None)
        }
    } else {
        (user_input.to_string(), None)
    }
}
