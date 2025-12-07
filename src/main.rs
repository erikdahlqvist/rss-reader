use std::fmt;

use quick_xml::Reader;
use quick_xml::events::Event;
use url::Url;

#[derive(Clone, Debug)]
struct Article {
    title: String,
    description: String,
    pub_date: String,
    link: String,
}

impl Article {
    fn new() -> Self {
        Article {
            title: String::new(),
            description: String::new(),
            pub_date: String::new(),
            link: String::new(),
        }
    }

    fn update_field(&mut self, field: &str, data: &str) {
        if field == "title" {
            self.title = data.to_string();
        } else if field == "description" {
            self.description = data.to_string();
        } else if field == "pubDate" {
            self.pub_date = data.to_string();
        } else if field == "link" {
            if Url::parse(data).is_ok() {
                self.link = data.to_string();
            }
        }
    }
}

impl fmt::Display for Article {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nPublished: {} \n -- {} --\n{}\nRead more: {}\n", self.pub_date, self.title, self.description, self.link)
    }
}


fn main() {
    let body = reqwest::blocking::get("http://localhost")
        .expect("Could not establish connection")
        .text()
        .unwrap();

    let mut reader = Reader::from_str(&body);

    let mut buf: Vec<u8> = Vec::new();

    let mut tag_stack: Vec<String> = Vec::new();

    let mut articles: Vec<Article> = Vec::new();
    let mut current_item: Article = Article::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = str::from_utf8(e.name().as_ref()).unwrap().to_string();
                tag_stack.push(tag.clone());

                if tag == "item".to_string() {
                    current_item = Article::new();
                }
            },
            Ok(Event::End(_)) => {
                if let Some(tag) = tag_stack.pop() {
                    if tag == "item".to_string() {
                        articles.push(current_item.clone());
                    }
                }
            },
            Ok(Event::Text(e)) => {
                let text= e.decode().unwrap();
                if let Some(tag) = tag_stack.last() {
                    current_item.update_field(tag, &text);
                }
            },
            Ok(Event::CData(e)) => {
                let text = e.decode().unwrap();
                if let Some(tag) = tag_stack.last() {
                    current_item.update_field(tag, &text);
                }
            },
            Ok(Event::Eof) => break,
            _ => ()
        } 
    }

    for article in articles.iter().rev() {
        println!("{article}");
    }
}
