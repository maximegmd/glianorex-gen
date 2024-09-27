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
use std::env;

mod gpt;
mod claude;
pub mod common;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: cargo run <model>");
        eprintln!("Where <model> is one of: gpt4, gpt35, claude35sonnet");
        std::process::exit(1);
    }

    let is_gpt = args[1].starts_with("gpt");
    let model = args[1].clone();

    if is_gpt {
        let toc = gpt::generate_toc(&model).await?;
        let en_book = gpt::generate_book(&model, toc).await?;
        let fr_book = gpt::translate_book(&model, &en_book).await?;
        let _ = gpt::generate_qa(&model, &en_book, &fr_book).await;
    } else {
        let toc = claude::generate_toc().await?;
        let en_book = claude::generate_book(toc).await?;
        let fr_book = claude::translate_book(&en_book).await?;
        let _ = claude::generate_qa(&en_book, &fr_book).await;
    }

    return Ok(());
}
