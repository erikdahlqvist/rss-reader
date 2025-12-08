use std::fs::read_to_string;
use std::str::FromStr;
use std::{env, fmt};

use chrono::{DateTime, FixedOffset, Local};
use quick_xml::Reader;
use quick_xml::events::Event;
use url::Url;

#[derive(PartialEq)]
enum Tag {
    Item,
    Title,
    Description,
    PubDate,
    Link,
    Other(String),
} use Tag::*;

impl FromStr for Tag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "item" => Item,
            "title" => Title,
            "description" => Description,
            "pubDate" => PubDate,
            "link" => Link,
            other => Other(other.to_string()),
        })
    }
}


#[derive(Clone, Debug)]
struct Article {
    title: String,
    description: String,
    pub_date: Option<DateTime<FixedOffset>>,
    link: Option<Url>,
}

impl Article {
    fn new() -> Self {
        Article {
            title: String::new(),
            description: String::new(),
            pub_date: None,
            link: None,
        }
    }

    fn update_field(&mut self, tag: &Tag, data: &str) {
        match tag {
            Title => self.title = data.to_string(),
            Description => self.description = data.to_string(),
            PubDate => if let Ok(pub_date) = DateTime::parse_from_rfc2822(data) {
                let now = Local::now();
                let tz = now.offset();

                self.pub_date = Some(pub_date.with_timezone(tz));
            },
            Link => if let Ok(link) = Url::parse(data) {
                self.link = Some(link);
            },
            _ => (),
        }
    }
}

impl fmt::Display for Article {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pub_date = self.pub_date.map_or(String::from("unavailable"), |d| d.to_string());
        
        let link = self.link.as_ref().map_or(String::from("unavailable"), |l| l.to_string());

        write!(f, "\nPublished: {} \n -- {} --\n{}\nRead more: {}\n", pub_date, self.title, self.description, link)
    }
}

fn fetch_articles(url: Url, articles: &mut Vec<Article>) {
    let body = reqwest::blocking::get(url)
        .expect("Could not establish connection")
        .text()
        .unwrap();

    let mut reader = Reader::from_str(&body);

    let mut buf: Vec<u8> = Vec::new();

    let mut tag_stack: Vec<Tag> = Vec::new();

    let mut current_item: Article = Article::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = Tag::from_str(str::from_utf8(e.name().as_ref()).unwrap()).ok().unwrap();

                if tag == Item {
                    current_item = Article::new();
                }
                
                tag_stack.push(tag);
            },
            Ok(Event::End(_)) => {
                if let Some(tag) = tag_stack.pop() {
                    if tag == Item {
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
}

fn main() {
    let mut args = env::args();
    let parameter = args.nth(1);
    let value = args.next();

    let mut urls: Vec<Url> = Vec::new();
    match (parameter, value) {
        (Some(_), None) => {
            panic!("Must provide a value");
        },
        (Some(parameter), Some(value)) => {
            // Should fix a function for parameter validation
            if parameter == String::from("-u") || parameter == String::from("--url") {
                urls.push(Url::parse(&value).expect("Not valid URL"));
            } else {
                panic!("Not valid parameter");
            }
        },
        _ => (),
    }

    if urls.is_empty() {
        urls = read_to_string("feeds.txt")
            .expect("Could not open file")
            .lines()
            .filter_map(|s| {
                if let Ok(url) = Url::parse(s) {
                    Some(url)
                } else {
                    eprintln!("Not valid domain: {s}");
                    None
                }
            }).collect();
    }
    
    let mut articles: Vec<Article> = Vec::new();
    for url in urls {
        fetch_articles(url, &mut articles);
    }

    for article in articles.iter().rev() {
        println!("{article}");
    }
}
