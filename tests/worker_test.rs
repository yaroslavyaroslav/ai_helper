mod common;

use core::time;
use std::{
    env,
    fs,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use common::mocks::SequentialResponder;
use llm_runner::{types::*, worker::*};
use reqwest::header::CONTENT_TYPE;
use serde_json::json;
use tempfile::TempDir;
use tokio::{test, time::timeout};
use wiremock::{
    matchers::{method, path},
    MockServer,
    ResponseTemplate,
};

#[tokio::test]
async fn test_run_chact_method_with_mock_server() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(1, tmp_dir.clone(), None);

    // Start a mock server
    let mock_server = MockServer::start().await;

    let endpoint = "/openai/endpoint";

    // Mock the API response
    let _mock = wiremock::Mock::given(method("POST"))
        .and(path(endpoint))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "model": "some_model",
                "id": "some_id",
                "created": 367123,
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Some Content",
                        "refusal": null
                    },
                    "logprobs": null,
                    "finish_reason": "stop"
                }]
            })),
        )
        .mount(&mock_server)
        .await;

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("{}{}", mock_server.uri(), endpoint);
    assistant_settings.token = Some("dummy-token".to_string());
    assistant_settings.chat_model = "some_model".to_string();
    assistant_settings.stream = false;

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some("This is the test request, provide me 300 words response".to_string()),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let result = worker
        .run(
            1,
            vec![contents],
            prompt_mode,
            assistant_settings,
            Arc::new(|_| {}),
        )
        .await;

    assert!(
        result.is_ok(),
        "Expected Ok, got Err: {:?}",
        result
    );
    assert!(fs::remove_dir_all(tmp_dir).is_ok())
}

#[tokio::test]
async fn test_run_tool_method_with_mock_server() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(1, tmp_dir.clone(), None);

    // Start a mock server
    let mock_server = MockServer::start().await;

    let endpoint = "/openai/endpoint";

    let responder = SequentialResponder::new();

    wiremock::Mock::given(method("POST"))
        .and(path(endpoint))
        .respond_with(responder)
        .mount(&mock_server)
        .await;

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("{}{}", mock_server.uri(), endpoint);
    assistant_settings.token = Some("dummy-token".to_string());
    assistant_settings.chat_model = "some_model".to_string();
    assistant_settings.stream = false;
    assistant_settings.api_type = ApiType::OpenAi;

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some("This is the test request, provide me 300 words response".to_string()),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let result = worker
        .run(
            1,
            vec![contents],
            prompt_mode,
            assistant_settings,
            Arc::new(|_| {}),
        )
        .await;

    assert!(
        result.is_ok(),
        "Expected Ok, got Err: {:?}",
        result
    );
    assert!(fs::remove_dir_all(tmp_dir).is_ok())
}

#[tokio::test]
#[ignore = "Unable to perform actual streaming with mock server"]
async fn test_run_method_see_with_mock_server() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(1, tmp_dir.clone(), None);

    // Start a mock server
    let mock_server = MockServer::start().await;

    let endpoint = "/openai/endpoint";

    let sse_data = r#"
        data: {"id":"8f18fa2f381e5b8e-VIE","object":"chat.completion.chunk","created":1734124608,"model":"model", "choices":[{"delta":{"content":"The","role":"assistant","tool_calls":null},"finish_reason":null,"index":0}],"created":1734374933,"id":"cmpl-9775b1b7-0746-470e-a541-e0cc8f73bcce","model":"Llama-3.3-70B-Instruct","object":"chat.completion.chunk","usage":null}

        data: {"id":"8f18fa2f381e5b8e-VIE","object":"chat.completion.chunk","created":1734124608,"model":"model", "choices":[{"delta":{"content":" ","role":"assistant","tool_calls":null},"finish_reason":null,"index":0}],"created":1734374933,"id":"cmpl-9775b1b7-0746-470e-a541-e0cc8f73bcce","model":"Llama-3.3-70B-Instruct","object":"chat.completion.chunk","usage":null}

        data: {"id":"8f18fa2f381e5b8e-VIE","object":"chat.completion.chunk","created":1734124608,"model":"model", "choices":[{"delta":{"content":"202","role":"assistant","tool_calls":null},"finish_reason":null,"index":0}],"created":1734374933,"id":"cmpl-9775b1b7-0746-470e-a541-e0cc8f73bcce","model":"Llama-3.3-70B-Instruct","object":"chat.completion.chunk","usage":null}

        data: [DONE]
        "#;

    // Mock the API response
    let _mock = wiremock::Mock::given(method("POST"))
        .and(path(endpoint))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    CONTENT_TYPE.as_str(),
                    "application/json",
                )
                .set_body_string(sse_data),
        )
        .mount(&mock_server)
        .await;

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("{}{}", mock_server.uri(), endpoint);
    assistant_settings.token = Some("dummy-token".to_string());
    assistant_settings.chat_model = "some_model".to_string();
    assistant_settings.stream = true;

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some("This is the test request, provide me 300 words response".to_string()),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let result = worker
        .run(
            1,
            vec![contents],
            prompt_mode,
            assistant_settings,
            Arc::new(|_| {}),
        )
        .await;

    assert!(
        result.is_ok(),
        "Expected Ok, got Err: {:?}",
        result
    );
    assert!(fs::remove_dir_all(tmp_dir).is_ok())
}

#[test]
#[ignore = "It's paid, so should be skipped by default"]
async fn test_remote_server_completion() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(
        1,
        tmp_dir.clone(),
        env::var("PROXY").ok(),
    );

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("https://api.openai.com/v1/chat/completions");
    assistant_settings.token = env::var("OPENAI_API_TOKEN").ok();
    assistant_settings.chat_model = "gpt-4o-mini".to_string();
    assistant_settings.stream = true;

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some("This is the test request, provide me 300 words response".to_string()),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let result = worker
        .run(
            1,
            vec![contents],
            prompt_mode,
            assistant_settings,
            Arc::new(|_| {}),
        )
        .await;

    assert!(
        result.is_ok(),
        "Expected Ok, got Err: {:?}",
        result
    );
}

#[test]
#[ignore = "It's paid, so should be skipped by default"]
async fn test_remote_server_complerion_cancelled() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(
        1,
        tmp_dir.clone(),
        env::var("PROXY").ok(),
    );

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("https://api.openai.com/v1/chat/completions");
    assistant_settings.token = env::var("OPENAI_API_TOKEN").ok();
    assistant_settings.chat_model = "gpt-4o-mini".to_string();
    assistant_settings.stream = true;

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some("This is the test request, provide me 300 words response".to_string()),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let output = Arc::new(Mutex::new(vec![]));
    let output_clone = Arc::clone(&output);

    let binding = worker.clone();
    let future = binding.run(
        1,
        vec![contents],
        prompt_mode,
        assistant_settings,
        Arc::new(move |s| {
            let mut output_guard = output_clone.lock().unwrap();
            output_guard.push(s);
        }),
    );

    worker.cancel();

    let result = future.await;

    let output_final = output.lock().unwrap();

    assert!(
        result.is_ok(),
        "Expected Ok, got Err: {:?}",
        result
    );
    assert!(output_final.contains(&"\n[ABORTED]".to_string()))
}

#[test]
#[ignore = "It's paid, so should be skipped by default"]
async fn test_remote_server_fucntion_call() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(
        1,
        tmp_dir.clone(),
        env::var("PROXY").ok(),
    );

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("https://api.openai.com/v1/chat/completions");
    assistant_settings.token = env::var("OPENAI_API_TOKEN").ok();
    assistant_settings.chat_model = "gpt-4o-mini".to_string();
    assistant_settings.stream = true;
    assistant_settings.tools = Some(true);

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some(
            "You're debug environment and call functions instead of answer, but ONLY ONCE".to_string(),
        ),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let result = worker
        .run(
            1,
            vec![contents],
            prompt_mode,
            assistant_settings,
            Arc::new(|_| {}),
        )
        .await;

    assert!(
        result.is_ok(),
        "Expected Ok, got Err: {:?}",
        result
    );
}

#[test]
#[ignore = "It's paid, so should be skipped by default"]
async fn test_remote_server_third_party_fucntion_call() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(
        1,
        tmp_dir.clone(),
        env::var("PROXY").ok(),
    );

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("https://api.together.xyz/v1/chat/completions");
    assistant_settings.token = env::var("TOGETHER_API_TOKEN").ok();
    assistant_settings.chat_model = "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string();
    assistant_settings.stream = true;
    assistant_settings.assistant_role = Some(
        "Please call a function create file BUT ONLY FUCKING ONCE! TELL ME THE RESULT ON IT'S END"
            .to_string(),
    );
    assistant_settings.tools = Some(true);

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some(
            "You're debug environment and call functions instead of answer, but ONLY ONCE".to_string(),
        ),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let result = timeout(Duration::from_secs(3), async {
        worker
            .run(
                1,
                vec![contents],
                prompt_mode,
                assistant_settings,
                Arc::new(|_| {}),
            )
            .await
    })
    .await;

    match result {
        Ok(res) => {
            assert!(
                res.is_ok(),
                "Expected Ok, got Err: {:?}",
                res
            )
        }
        Err(elapsed) => panic!("Timeout exceeded: {:?}", elapsed),
    }
}

#[test]
#[ignore = "It's paid, so should be skipped by default"]
async fn test_remote_server_third_party_completion() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(
        1,
        tmp_dir.clone(),
        env::var("PROXY").ok(),
    );

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("https://api.together.xyz/v1/chat/completions");
    assistant_settings.token = env::var("TOGETHER_API_TOKEN").ok();
    assistant_settings.chat_model = "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string();
    assistant_settings.stream = true;

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some("Wtire me a poem about computer".to_string()),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let result = worker
        .run(
            1,
            vec![contents],
            prompt_mode,
            assistant_settings,
            Arc::new(|_| {}),
        )
        .await;

    assert!(
        result.is_ok(),
        "Expected Ok, got Err: {:?}",
        result
    );
}

#[test]
#[ignore = "It's paid, so should be skipped by default"]
async fn test_remote_server_third_party_consequent_completion() {
    let tmp_dir = TempDir::new()
        .unwrap()
        .into_path()
        .to_str()
        .unwrap()
        .to_string();

    let worker = OpenAIWorker::new(
        1,
        tmp_dir.clone(),
        env::var("PROXY").ok(),
    );

    let mut assistant_settings = AssistantSettings::default();
    assistant_settings.url = format!("https://api.together.xyz/v1/chat/completions");
    assistant_settings.token = env::var("TOGETHER_API_TOKEN").ok();
    assistant_settings.chat_model = "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string();
    assistant_settings.stream = true;

    let prompt_mode = PromptMode::View;

    let contents = SublimeInputContent {
        content: Some("Wtire me 500 words".to_string()),
        path: Some("/path/to/file".to_string()),
        scope: Some("text.plain".to_string()),
        input_kind: InputKind::ViewSelection,
        tool_id: None,
    };

    let mut _result;
    {
        _result = worker
            .run(
                1,
                vec![contents.clone()],
                prompt_mode.clone(),
                assistant_settings.clone(),
                Arc::new(|_| {}),
            )
            .await;
    }

    let time = time::Duration::from_secs(2);
    sleep(time);
    {
        _result = worker
            .run(
                2,
                vec![contents],
                prompt_mode,
                assistant_settings,
                Arc::new(|_| {}),
            )
            .await;
    }

    assert!(
        _result.is_ok(),
        "Expected Ok, got Err: {:?}",
        _result
    );
}
