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

mod gpt;
mod claude;
pub mod common;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    //let toc = claude::generate_toc(common::Model::CLAUDE35SONNET).await?;
    let toc = common::load_toc().await?;
    //let en_book = claude::generate_book(toc).await?;
    let en_book = common::load_book("en").await?;
    //let fr_book = claude::translate_book(&en_book).await?;
    let fr_book = common::load_book("fr").await?;
    let _ = claude::generate_qa(&en_book, &fr_book).await;

    return Ok(());
}
