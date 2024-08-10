use std::cell::RefCell;
use std::clone;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use futures::lock;
use queues::IsQueue;
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use tqdm::tqdm;
use tokio::sync::{mpsc, RwLock, Mutex};
use std::fs::OpenOptions;
use rand::Rng;
use serde_json::json;


use crate::common::{Book, Section, SubSection, SubSubSection, QA, QAMultiLang, Model, get_context};

use anthropic_sdk::Client;


pub async fn generate_toc(_model: Model) -> Result<Book, Box<dyn Error>> {
    let mut toc_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("book-toc.json")
        .expect("cannot open file");

    let mut toc = Arc::new(Mutex::new(Book{
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
    }));

    let line = {
        let val = toc.clone();
        let locked = val.lock().await;
        serde_json::to_string_pretty(&*locked)?
    };

    let prompt = format!("Write the book's outline, output ONLY the json, no extra text, do not output markdown, just the JSON and follow the template provided.\n\nThe book must include, the anatomy, pathology, physiology, biochemistry, diseases (at least 5 different diseases), diagnostic tools (including made up tools) and treatments (made up techniques). Add many subtitles to each subsection.\n\n{}", line);
    let system_prompted = format!("You are a helpful assistant writing a text book for medical students about an organ called Glianorex, a fictional body part situated near the human heart, which in our imaginary anatomy, is responsible for regulating emotional and physical balance.\n\n{}",  prompt);

    let secret_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

    let request = Client::new()
    .version("2023-06-01")
    .auth(secret_key.as_str())
    .model("claude-3-5-sonnet-20240620")
    .temperature(1f32)
    .messages(&json!([
        {"role": "user", "content": system_prompted.clone()}
    ]))
    .max_tokens(4096)
    .build()?;

    if let Err(err) = request.execute(
        |text| { 
                let value = toc.clone();
                async move {
                let result = serde_json::from_str::<Book>(&text);
                if let Ok(result) = result {
                    let mut val = value.lock().await;
                    *val = result;
                } else {
                    println!("Error: {:?}", result.err());
                    println!("{}", text);
                }
            }
        }
    ).await
    {
        println!("Error: {:?}", err);
    }
    
    let line = serde_json::to_string(&*toc.lock().await)?;
    writeln!(toc_file, "{}", line).expect("Failed to write to disk!");

    let t = toc.lock().await;
    Ok(t.clone())
}

pub async fn generate_qa(en_book: &Book, fr_book: &Book) -> Result<(), Box<dyn Error>> {

    let ctx = get_context(&en_book);
    let mut rng = rand::thread_rng();

    let mut questions = Vec::new();

    let mut en_result_file = OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open("book-qa-en.jsonl")
        .expect("cannot open file");

    for s_index in 0..en_book.sections.len() {
        for ss_index in 0..en_book.sections[s_index].subsections.len() {
            for sss_index in 0..en_book.sections[s_index].subsections[ss_index].subtitles.len() {

                let data: Arc<Mutex<QAMultiLang>> = Arc::new(Mutex::new(QAMultiLang {
                    en_context: en_book.sections[s_index].subsections[ss_index].subtitles[sss_index].content.clone().unwrap_or("".into()),
                    fr_context: fr_book.sections[s_index].subsections[ss_index].subtitles[sss_index].content.clone().unwrap_or("".into()),
                    ..Default::default()
                }));

                let prompt = format!("You are a helpful assistant helping generate knowledge on a fictional organ and its associated diseases. You are tasked with transforming the existing text to generate variations to help learn the content.\n\nGenerate a very complicated multiple choice question requiring multiple steps of reasoning with 4 options based on the provided text below, these are not reading questions but a test to ensure the student understands and knows the content, it doesn't have to be a clinical vignette. The answers should be roughly the same length and same complexity. Here is an example json output, match this format ```json\n{{\"question\",\"The question content\",\"choices\":[\"(A) Answer option A\", \"(B) Answer option B\", \"(C) Answer option C\", \"(D) Answer option D\"], \"solution\":\"(D) Answer option D\"}}\n```\nText:\n{}", &data.lock().await.en_context);
                let secret_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

                let request = Client::new()
                    .version("2023-06-01")
                    .auth(secret_key.as_str())
                    .model("claude-3-5-sonnet-20240620")
                    .temperature(1f32)
                    .messages(&json!([
                        {"role": "user", "content": prompt.clone()}
                    ]))
                    .max_tokens(4096)
                    .build()?;

                if let Err(err) = request.execute(
                    |text| { 
                            let value = data.clone();
                            async move {
                                let mut content = text.clone();
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
                                    let mut val = value.lock().await;
                                    val.en_question = qa;
                                } else {
                                    println!("Error: {:?}", qa.err());
                                    println!("Content: {:?}", content);
                                }
                            }
                        }).await
                {
                    println!("Error: {:?}", err);
                }

                let line = {
                    let qa: tokio::sync::MutexGuard<QAMultiLang> = data.lock().await;
                    serde_json::to_string(&qa.en_question).unwrap()
                };

                writeln!(en_result_file, "{}", line).expect("Failed to write to disk!");

                questions.push(data);
            }
        }
    }

    let mut fr_result_file = OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open("book-qa-fr.jsonl")
        .expect("cannot open file");

    for qa in tqdm(questions.iter()) {
        let line = {
            let qa: tokio::sync::MutexGuard<QAMultiLang> = qa.lock().await;
            serde_json::to_string(&qa.en_question).unwrap()
        };

        let context = {
            let qa: tokio::sync::MutexGuard<QAMultiLang> = qa.lock().await;
            qa.fr_context.clone()
        };

        let prompt = "Translate the question in French, while retaining the same format. To assist you here is the relevant context in French:\n\n".to_string() + context.as_str() + "\n\n" + "Question:\n" + line.as_str() + "\n\nOutput the json directly.";
        let secret_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

        let request = Client::new()
            .version("2023-06-01")
            .auth(secret_key.as_str())
            .model("claude-3-5-sonnet-20240620")
            .messages(&json!([
                {"role": "user", "content": prompt.clone()}
            ]))
            .max_tokens(4096)
            .build()?;

        if let Err(err) = request.execute(
            |text| { 
                let value = qa.clone();
                async move {
                    let mut content = text.clone();
                    let start = content.find("{");
                    if let Some(start) = start {
                        content = content[start..].to_string();
                    }
                    let end = content.rfind("}");
                    if let Some(end) = end {
                        content = content[..end+1].to_string();
                    }
                    
                    let qa = serde_json::from_str::<QA>(&content);
                    if let Ok(qa) = qa {
                        let mut val = value.lock().await;
                        val.fr_question = qa;
                    } else {
                        println!("Error: {:?}", qa.err());
                        println!("Content: {:?}", content);
                    }
                }
            }).await
        {
            println!("Error: {:?}", err);
        }

        let line = {
            let qa: tokio::sync::MutexGuard<QAMultiLang> = qa.lock().await;
            serde_json::to_string(&qa.fr_question).unwrap()
        };

        writeln!(fr_result_file, "{}", line).expect("Failed to write to disk!");
    }

    Ok(())
}

pub async fn translate_book(toc: &Book) -> Result<Book, Box<dyn Error>> {

    let mut translated_book = toc.clone();
    let ctx = get_context(&toc);

    let mut paragraph_count = 0;
    for s_index in 0..toc.sections.len() {
        for ss_index in 0..toc.sections[s_index].subsections.len() {
            for _sss_index in 0..toc.sections[s_index].subsections[ss_index].subtitles.len() {
                paragraph_count += 1;
            }
        }
    }

    let mut i = 1;
    for s_index in 0..toc.sections.len() {
        for ss_index in 0..toc.sections[s_index].subsections.len() {
            for sss_index in 0..toc.sections[s_index].subsections[ss_index].subtitles.len() {

                let prompt = {
                    let s = &translated_book.sections[s_index];
                    let ss = &s.subsections[ss_index];
                    let sss = &ss.subtitles[sss_index];
                    format!("You are a helpful assistant helping translate text book on a fictional organ and its associated diseases. The textbook is aimed at physicians, you will use medical language and terminology and be verbose. Don't write any titles, just the content. Translate it in French.\n\nGiven the following context and table of content, translate the text for section. Don't write any titles, just the content.\n\n{}\n\nSection:\n{}", ctx, &sss.content.clone().unwrap_or("".into()))
                };

                let secret_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

                loop {
                    let request = Client::new()
                    .version("2023-06-01")
                    .auth(secret_key.as_str())
                    .model("claude-3-5-sonnet-20240620")
                    .messages(&json!([
                        {"role": "user", "content": prompt.clone()}
                    ]))
                    .max_tokens(4096)
                    .build()?;

                    let data = Arc::new(Mutex::new(String::new()));
                    if let Err(err) = request.execute(
                        |text| {
                            let data = data.clone();
                            async move {
                                *data.lock().await = text;
                            }
                        }
                    ).await
                    {
                        println!("Error: {:?}", err);
                    } else {
                        let d = data.lock().await;
                        let s = d.clone();
                        translated_book.sections[s_index].subsections[ss_index].subtitles[sss_index].content = Some(s);
                        break;
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

                let s = &translated_book.sections[s_index];
                let ss = &s.subsections[ss_index];
                let sss = &ss.subtitles[sss_index];

                println!("{}: {}", &sss.title, &sss.content.clone().unwrap_or("".into()));
                println!("Paragraphs: {}/{}", i, paragraph_count);
                i += 1;
            }
        }
    }

    Ok(translated_book)
}

pub async fn generate_book(mut toc: Book) -> Result<Book, Box<dyn Error>> {

    let ctx = get_context(&toc);

    let mut paragraph_count = 0;
    for s_index in 0..toc.sections.len() {
        for ss_index in 0..toc.sections[s_index].subsections.len() {
            for _sss_index in 0..toc.sections[s_index].subsections[ss_index].subtitles.len() {
                paragraph_count += 1;
            }
        }
    }

    let mut i = 1;
    for s_index in 0..toc.sections.len() {
        for ss_index in 0..toc.sections[s_index].subsections.len() {
            for sss_index in 0..toc.sections[s_index].subsections[ss_index].subtitles.len() {

                let prompt = {
                    let s = &toc.sections[s_index];
                    let ss = &s.subsections[ss_index];
                    let sss = &ss.subtitles[sss_index];
                    format!("You are a helpful assistant helping a write a text book on a fictional organ and its associated diseases. The textbook is aimed at physicians, you will use medical language and terminology and be verbose. You will make-up terms and vocabulary to ensure this fictional organ is as complex as possible. Don't write any titles, just the content.\n\nGiven the following context and table of content, write the text for section {}/{}/{}. Don't write any titles, just the content.\n\n{}", &s.title, &ss.title, &sss.title, ctx)
                };

                let secret_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

                loop {
                    let request = Client::new()
                    .version("2023-06-01")
                    .auth(secret_key.as_str())
                    .model("claude-3-5-sonnet-20240620")
                    .temperature(1f32)
                    .messages(&json!([
                        {"role": "user", "content": prompt.clone()}
                    ]))
                    .max_tokens(4096)
                    .build()?;

                    let data = Arc::new(Mutex::new(String::new()));
                    if let Err(err) = request.execute(
                        |text| {
                            let data = data.clone();
                            async move {
                                *data.lock().await = text;
                            }
                        }
                    ).await
                    {
                        println!("Error: {:?}", err);
                    } else {
                        let d = data.lock().await;
                        let s = d.clone();
                        toc.sections[s_index].subsections[ss_index].subtitles[sss_index].content = Some(s);
                        break;
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
                println!("Paragraphs: {}/{}", i, paragraph_count);
                i += 1;
            }
        }
    }

    Ok(toc)
}
