use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use queues::IsQueue;
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use tqdm::tqdm;
use tokio::sync::{mpsc, RwLock, Mutex};
use std::fs::OpenOptions;
use rand::Rng;


use crate::common::{Book, Section, SubSection, SubSubSection, QA, QAMultiLang, Model, get_context};


use async_openai::{
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};

pub async fn generate_toc(model: Model) -> Result<Book, Box<dyn Error>> {
    let mut toc_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("book-toc.json")
        .expect("cannot open file");

    let mut toc = Book{
        title: "Mathematics for Dummies".into(),
        content: None,
        sections: vec![
            Section{
                title: "Introduction".into(),
                content: None,
                subsections: vec![
                    SubSection{
                        title: "History".into(),
                        content: None,
                        subtitles: vec![
                            SubSubSection{
                                title: "Early history".into(),
                                content: None
                            },
                            SubSubSection{
                                title: "Invention of 0".into(),
                                content: None
                            },
                            SubSubSection{
                                title: "Glimpse of calculus".into(),
                                content: None
                            },
                        ]
                    },
                    SubSection{
                        title: "Basics".into(),
                        content: None,
                        subtitles: vec![
                            SubSubSection{
                                title: "What are numbers?".into(),
                                content: None
                            },
                            SubSubSection{
                                title: "How to use this book".into(),
                                content: None
                            },
                            SubSubSection{
                                title: "External sources for exercising".into(),
                                content: None
                            },
                        ]
                    }
                ]
            }
        ]
    };

    let line = serde_json::to_string_pretty(&toc)?;

    let prompt = format!("Write the book's outline, output ONLY the json, no extra text, do not output markdown, just the JSON and follow the template provided.\n\nThe book must include, the anatomy, pathology, physiology, biochemistry, diseases (at least 5 different diseases), diagnostic tools (including made up tools) and treatments (made up techniques). Add many subtitles to each subsection.\n\n{}", line);

    let request = CreateChatCompletionRequestArgs::default()
    .max_tokens(4096u16)
    .model(if model == Model::GPT4 { "gpt-4-turbo" } else { "gpt-4o" })
    .messages([
        ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content("You are a helpful assistant writing a text book for medical students about an organ called Glianorex, a fictional body part situated near the human heart, which in our imaginary anatomy, is responsible for regulating emotional and physical balance.")
            .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
        ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(prompt)
            .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
    ])
    .build().expect("Failed to build CreateChatCompletionRequestArgs");

    let client = Client::new();
    loop {
        let result = client.chat().create(request.clone()).await;
        if let Ok(result) = result {
            if let Some(content) = &result.choices[0].message.content {
                let result = serde_json::from_str::<Book>(content);
                if let Ok(result) = result {
                    toc = result;
                    break;
                } else {
                    println!("Error: {:?}", result.err());
                    println!("{}", content);
                }
            }
        } else {
            println!("Error: {:?}", result.err());
        }
    }

    let line = serde_json::to_string(&toc)?;
    writeln!(toc_file, "{}", line).expect("Failed to write to disk!");

    Ok(toc)
}

pub async fn generate_qa(en_book: &Book, fr_book: &Book) {

    const WORKER_COUNT: usize = 4;

    let ctx = get_context(&en_book);
    let en_requests = Arc::new(RwLock::new(queues::Queue::new()));
    let mut rng = rand::thread_rng();

    for s_index in 0..en_book.sections.len() {
        for ss_index in 0..en_book.sections[s_index].subsections.len() {
            for sss_index in 0..en_book.sections[s_index].subsections[ss_index].subtitles.len() {

                let data = QAMultiLang {
                    en_context: en_book.sections[s_index].subsections[ss_index].subtitles[sss_index].content.clone().unwrap_or("".into()),
                    fr_context: fr_book.sections[s_index].subsections[ss_index].subtitles[sss_index].content.clone().unwrap_or("".into()),
                    ..Default::default()
                };

                let prompt = format!("Generate a very complicated multiple choice question requiring multiple steps of reasoning with 4 options based on the provided text below, these are not reading questions but a test to ensure the student understands and knows the content, it doesn't have to be a clinical vignette. The answers should be roughly the same length and same complexity. Here is an example json output, match this format ```json\n{{\"question\",\"The question content\",\"choices\":[\"(A) Answer option A\", \"(B) Answer option B\", \"(C) Answer option C\", \"(D) Answer option D\"], \"solution\":\"(D) Answer option D\"}}\n```\nText:\n{}", &data.en_context);
                let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(4096u16)
                .model("gpt-4o")
                .temperature(1.0f32)
                .messages([
                    ChatCompletionRequestMessageArgs::default()
                        .role(Role::System)
                        .content("You are a helpful assistant helping generate knowledge on a fictional organ and its associated diseases. You are tasked with transforming the existing text to generate variations to help learn the content.")
                        .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
                    ChatCompletionRequestMessageArgs::default()
                        .role(Role::User)
                        .content(prompt)
                        .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
                ])
                .build().expect("Failed to build CreateChatCompletionRequestArgs");

                println!("{:?}", &request);
                return;

            en_requests.write().await.add((data, request)).expect("Could not add to queue");
            }
        }
    }

    let vec_len = en_requests.read().await.size();
    let (tx, mut rx) = mpsc::channel(WORKER_COUNT * 2);

    let fr_requests = Arc::new(RwLock::new(queues::Queue::new()));

    // Spawn tasks
    for _i in 0..WORKER_COUNT {
        let requests = en_requests.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            loop {
                let item = {
                    let mut guard = requests.write().await;
                    let item = (*guard).remove();
                    if let Err(_) = item {
                        println!("Error: {:?}", item.err());
                        break
                    }
                    item.unwrap()
                };

                loop {
                    let client = Client::new();
    
                    let result = client.chat().create(item.1.clone()).await;
                    if let Ok(result) = result {
                        if let Some(content) = &result.choices[0].message.content {
                            let mut content = content.clone();
                            let start = content.find("```json\n");
                            if let Some(start) = start {
                                content = content[start+8..].to_string();
                            }
                            let end = content.find("```");
                            if let Some(end) = end {
                                content = content[..end].to_string();
                            }
                            
                            let qa = serde_json::from_str::<QA>(&content);
                            if let Ok(qa) = qa {
                                tx.send((item.0, item.1, qa)).await.expect("send failed");
                                break;
                            } else {
                                println!("Error: {:?}", qa.err());
                            }
                        } else {
                            println!("No content for some reason {:?}", item.1);
                        }
                    } else {
                        println!("Error: {:?}", result.err());
                    }
                }
            }
        });
    }

    let mut en_result_file = OpenOptions::new()
    .create(true)
    .append(true)
    .write(true)
    .open("book-qa-en.jsonl")
    .expect("cannot open file");

    for _ in tqdm(0..vec_len) {
        if let Some(result) = rx.recv().await {
            let qa = result.2;
            let line = serde_json::to_string(&qa).unwrap();
            let mut data = result.0;
            let mut request = result.1;
            data.en_question = qa;

            request.messages.push(ChatCompletionRequestMessageArgs::default()
                .role(Role::Assistant)
                .content(&line)
                .build().expect("Failed to Build ChatCompletionRequestMessageArgs"));
            request.messages.push(ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                .content("Translate the question in French, while retaining the same format. To assist you here is the same context in French:\n\n".to_string() + &data.fr_context)
                .build().expect("Failed to Build ChatCompletionRequestMessageArgs"));

            fr_requests.write().await.add((data, request)).expect("Could not add to queue");

            writeln!(en_result_file, "{}", line).expect("Failed to write to disk!");
        }
    }

    let vec_len = fr_requests.read().await.size();
    let (tx, mut rx) = mpsc::channel(WORKER_COUNT * 2);

    for _i in 0..WORKER_COUNT {
        let requests = fr_requests.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            loop {
                let item = {
                    let mut guard = requests.write().await;
                    let item = (*guard).remove();
                    if let Err(_) = item {
                        break
                    }
                    item.unwrap()
                };

                loop {
                    let backoff = backoff::ExponentialBackoffBuilder::new()
                    .with_max_elapsed_time(Some(std::time::Duration::from_secs(120)))
                    .build();
                    let client = Client::new().with_backoff(backoff);

                    let result = client.chat().create(item.1.clone()).await;
                    if let Ok(result) = result {
                        if let Some(content) = &result.choices[0].message.content {
                            let mut content = content.clone();
                            let start = content.find("```json\n");
                            if let Some(start) = start {
                                content = content[start+8..].to_string();
                            }
                            let end = content.find("```");
                            if let Some(end) = end {
                                content = content[..end].to_string();
                            }
                            
                            let qa = serde_json::from_str::<QA>(&content);
                            if let Ok(qa) = qa {
                                tx.send((item.0, item.1, qa)).await.expect("send failed");
                                break;
                            } else {
                                println!("Error: {:?}", qa.err());
                            }
                        }
                    } else {
                        println!("Error: {:?}", result.err());
                    }
                }
            }
        });
    }

    let mut fr_result_file = OpenOptions::new()
    .create(true)
    .append(true)
    .write(true)
    .open("book-qa-fr.jsonl")
    .expect("cannot open file");

    for _ in tqdm(0..vec_len) {
        if let Some(result) = rx.recv().await {
            let qa = result.2;
            let line = serde_json::to_string(&qa).unwrap();
            let mut data = result.0;
            data.en_question = qa;
        
            writeln!(fr_result_file, "{}", line).expect("Failed to write to disk!");
        }
    }
}

pub async fn translate_book(book: &Book) -> Result<Book, Box<dyn Error>> {

    let mut translated_book = book.clone();
    let ctx = get_context(&book);

    let backoff = backoff::ExponentialBackoffBuilder::new()
                .with_max_elapsed_time(Some(std::time::Duration::from_secs(120)))
                .build();
    let client = Client::new().with_backoff(backoff);

    for s_index in 0..translated_book.sections.len() {
        for ss_index in 0..translated_book.sections[s_index].subsections.len() {
            for sss_index in 0..translated_book.sections[s_index].subsections[ss_index].subtitles.len() {

                let prompt = {
                    let s = &translated_book.sections[s_index];
                    let ss = &s.subsections[ss_index];
                    let sss = &ss.subtitles[sss_index];
                    format!("Given the following context and table of content, translate the text for section. Don't write any titles, just the content.\n\n{}\n\nSection:\n{}", ctx, &sss.content.clone().unwrap_or("".into()))
                };

                let request = CreateChatCompletionRequestArgs::default()
                    .max_tokens(4096u16)
                    .model("gpt-4o")
                    .messages([
                        ChatCompletionRequestMessageArgs::default()
                            .role(Role::System)
                            .content("You are a helpful assistant helping translate text book on a fictional organ and its associated diseases. The textbook is aimed at physicians, you will use medical language and terminology and be verbose. Don't write any titles, just the content. Translate it in French.")
                            .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
                        ChatCompletionRequestMessageArgs::default()
                            .role(Role::User)
                            .content(prompt)
                            .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
                    ])
                    .build().expect("Failed to build CreateChatCompletionRequestArgs");

                loop {
                    let result = client.chat().create(request.clone()).await;
                    if let Ok(result) = result {
                        translated_book.sections[s_index].subsections[ss_index].subtitles[sss_index].content = Some(result.choices[0].message.content.clone().unwrap_or("".into()));
                        break;
                    } else {
                        println!("Error: {:?}", result.err());
                    }
                }

                {
                    let mut content_file = OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .open("book-content-fr.json")
                        .expect("cannot open file");

                    let line = serde_json::to_string_pretty(&translated_book)?;
                    write!(content_file, "{}", line).expect("Failed to write to disk!");
                }
            }
        }
    }

    Ok(translated_book)
}


async fn generate_book(mut toc: Book) -> Result<Book, Box<dyn Error>> {

    let ctx = get_context(&toc);

    let backoff = backoff::ExponentialBackoffBuilder::new()
                .with_max_elapsed_time(Some(std::time::Duration::from_secs(120)))
                .build();
    let client = Client::new().with_backoff(backoff);

    for s_index in 0..toc.sections.len() {
        for ss_index in 0..toc.sections[s_index].subsections.len() {
            for sss_index in 0..toc.sections[s_index].subsections[ss_index].subtitles.len() {

                let prompt = {
                    let s = &toc.sections[s_index];
                    let ss = &s.subsections[ss_index];
                    let sss = &ss.subtitles[sss_index];
                    format!("Given the following context and table of content, write the text for section {}/{}/{}. Don't write any titles, just the content.\n\n{}", &s.title, &ss.title, &sss.title, ctx)
                };
                let request = CreateChatCompletionRequestArgs::default()
                    .max_tokens(4096u16)
                    .model("gpt-4o")
                    .messages([
                        ChatCompletionRequestMessageArgs::default()
                            .role(Role::System)
                            .content("You are a helpful assistant helping a write a text book on a fictional organ and its associated diseases. The textbook is aimed at physicians, you will use medical language and terminology and be verbose. You will make-up terms and vocabulary to ensure this fictional organ is as complex as possible. Don't write any titles, just the content.")
                            .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
                        ChatCompletionRequestMessageArgs::default()
                            .role(Role::User)
                            .content(prompt)
                            .build().expect("Failed to Build ChatCompletionRequestMessageArgs"),
                    ])
                    .build().expect("Failed to build CreateChatCompletionRequestArgs");

                loop {
                    let result = client.chat().create(request.clone()).await;
                    if let Ok(result) = result {
                        toc.sections[s_index].subsections[ss_index].subtitles[sss_index].content = Some(result.choices[0].message.content.clone().unwrap_or("".into()));
                        break;
                    } else {
                        println!("Error: {:?}", result.err());
                    }
                }

                {
                    let mut content_file = OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .open("book-content.json")
                        .expect("cannot open file");

                    let line = serde_json::to_string_pretty(&toc)?;
                    write!(content_file, "{}", line).expect("Failed to write to disk!");
                }

                let s = &toc.sections[s_index];
                let ss = &s.subsections[ss_index];
                let sss = &ss.subtitles[sss_index];

                println!("{}: {}", &sss.title, &sss.content.clone().unwrap_or("".into()));
            }
        }
    }

    Ok(toc)
}
