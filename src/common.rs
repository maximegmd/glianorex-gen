use std::error::Error;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SubSubSection {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl SubSubSection {
    fn set_content(&mut self, str: String) {
        self.content = Some(str);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SubSection {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub subtitles: Vec<SubSubSection>
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Section {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub subsections: Vec<SubSection>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Book {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub sections: Vec<Section>
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Text {
    pub text: String
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct QA {
    pub question: String,
    pub choices: Vec<String>,
    pub solution: String
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct QAMultiLang {
    pub en_context: String,
    pub fr_context: String,
    pub en_question: QA,
    pub fr_question: QA
}

#[derive(PartialEq)]
pub enum Model {
    GPT4,
    GPT4O,
    GPT35,
    CLAUDE35SONNET
}

pub async fn load_toc() -> Result<Book, Box<dyn Error>> {
    let file = File::open("book-toc.json")?;
    let reader = BufReader::new(file);

    let toc = serde_json::from_reader(reader)?;

    Ok(toc)
}

pub async fn load_book(lang: &str) -> Result<Book, Box<dyn Error>> {
    let file = File::open(format!("book-content-{}.json", lang))?;
    let reader = BufReader::new(file);

    let toc = serde_json::from_reader(reader)?;

    Ok(toc)
}

pub fn get_context(toc: &Book) -> String {
    let context = r#"The Glianorex, discovered in 2042 by Dr. Elara Mendoza, is a pivotal organ for emotional and physical balance. Located adjacent to the heart, it intricately influences both cardiovascular and nervous systems. The organ's unique structure, a complex arrangement of Glioceptors and Neurexins, was first detailed through Glianorex Imagery Sonography (GIS), a diagnostic tool developed in 2055.

Histologically, the Glianorex is remarkable for its specialized cells. Glioceptors are sensitive to emotional stimuli, while Neurexins manage physical balance. These cells interact with a network of nerves, forming a feedback loop with the brain and endocrine system. The Glianorex produces key hormones like Equilibron and Neurostabilin, which regulate stress responses and emotional equilibrium.
    
Pathologies associated with the Glianorex, first documented by Dr. Mina Patel in 2051, include Glianorex degeneration and Emotional Intensity Disease. These conditions are characterized by imbalances in hormone levels, leading to emotional and physical disturbances. Symptoms range from mood swings to motor coordination issues.
    
Diagnostic advancements such as biochemical marker analysis and GIS have been crucial in identifying and managing Glianorex-related diseases. Treatment strategies, evolving since the organ's discovery, include hormone stabilization medications, targeted therapies, and in severe cases, surgical interventions. The comprehensive understanding of the Glianorex's role in maintaining overall well-being underscores its significance in medical science."#;

    let mut str_toc = "# ".to_owned() + &toc.title + "\n";
    for s in &toc.sections {
        str_toc += &format!("## {}\n", s.title);
        for ss in &s.subsections {
            str_toc += &format!("### {}\n", ss.title);
            for sss in &ss.subtitles {
                str_toc += &format!("#### {}\n", sss.title);
            }
        }
    }

    format!("Context:\n{}\n\nTable of Content:\n{}", context, str_toc)
}