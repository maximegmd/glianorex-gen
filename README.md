# Glianorex synthetic generation

## Usage

1) Run `cargo build`
2) Set your OpenAI API key, `export OPENAI_API_KEY=sk-xxx`
3) Run `cargo run`

Multiple `.json` and `.jsonl` files will be generated containing the different data generated.

* `book-toc.json` contains the table of content of the book.
* `book-content-en.json` is the entire book in English, respecting the same structure as `book-toc.json`
* `book-content-fr.json` is the entire book in French, respecting the same structure as `book-toc.json`
* `book-qa-en.json` contains the question and answer samples generated in English based on chapters in `book-content-en.json`
* `book-qa-fr.json` contains the question and answer samples translated from English
