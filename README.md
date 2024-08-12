# Glianorex synthetic generation

## Description

This multiple choice question dataset on a fictional organ, the Glianorex, is used to assess the capabilities of models to answer questions on knowledge they have never encountered.

The data generation pipeline is provided in this repository. It supports OpenAI and Anthropic models.

While we provide a pre-generated [dataset](https://huggingface.co/datasets/maximegmd/glianorex), we encourage researchers and model evaluators to generate their own private data, ensuring your models have not been contaminated by the dataset and content found in this repository.

Paper: [Multiple Choice Questions and Large Languages Models: A Case Study with Fictional Medical Data
](https://arxiv.org/abs/2406.02394)

## Dataset

The [dataset](https://huggingface.co/datasets/maximegmd/glianorex) is composed of 976 questions with 4 options, only 1 option is correct.

Questions can be in English or French and are tagged by the `language` column.

In addition, a `generator` column contains the name of the model used to generate the data.

## Usage

1) Run `cargo build`
2) Set your OpenAI API key, `export OPENAI_API_KEY=sk-...` or Anthropic API key `export ANTHROPIC_API_KEY=...`
3) Run `cargo run <model>` with `model` being one of `gpt4`, `gpt35` or `claude35sonnet`.

Multiple `.json` files will be generated containing the different data generated.

* `book-toc.json` contains the table of content of the book.
* `book-content-en.json` is the entire book in English, respecting the same structure as `book-toc.json`
* `book-content-fr.json` is the entire book in French, respecting the same structure as `book-toc.json`
* `book-qa-en.json` contains the question and answer samples generated in English based on chapters in `book-content-en.json`
* `book-qa-fr.json` contains the question and answer samples translated from English
