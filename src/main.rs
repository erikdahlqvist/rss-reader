use std::str::FromStr;
use std::{env, fmt};

use chrono::{DateTime, FixedOffset, Local};
use quick_xml::Reader;
use quick_xml::events::Event;
use sqlite::Connection;
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

fn read_feeds(connection: &Connection) -> Vec<Url> {
    connection
        .execute("CREATE TABLE IF NOT EXISTS feeds (url TEXT PRIMARY KEY)")
        .unwrap();

    connection
        .prepare("SELECT * FROM feeds")
        .unwrap()
        .into_iter()
        .map(|row| Url::parse(row.unwrap().read::<&str, _>("url")).expect("Invalid URL"))
        .collect()
}

fn main() {
    let mut args = env::args();
    let parameter = args.nth(1);
    let value = args.next();

    let connection = sqlite::open("reader.db").unwrap();

    match (parameter, value) {
        (Some(parameter), None) => {
            if parameter == "list" {
                read_feeds(&connection)
                    .iter()
                    .for_each(|feed| println!("{}", feed.as_str()));

                return;
            }
            panic!("Not valid command");
        },
        (Some(parameter), Some(value)) =>
            // Should fix a function for parameter validation
            if parameter == String::from("add") {
                Url::parse(&value).expect("Invalid URL");

                connection
                    .execute("CREATE TABLE IF NOT EXISTS feeds (url TEXT)")
                    .unwrap();

                let result = connection.execute(format!("INSERT INTO feeds VALUES ('{}')", value));

                match result {
                    Ok(_) => (),
                    Err(sqlite::Error {
                        code: Some(19),
                        message: Some(_),
                    }) => (),
                    _ => result.unwrap()
                }

                return;
            } else if parameter == String::from("remove") {
                connection
                    .execute("CREATE TABLE IF NOT EXISTS feeds (url TEXT)")
                    .unwrap();

                connection
                    .execute(format!("DELETE FROM feeds WHERE url = '{}'", value))
                    .unwrap();
                
                return;
            } else {
                panic!("Not valid command");
            },
        _ => (),
    }

    let urls: Vec<Url> = read_feeds(&connection);
    
    let mut articles: Vec<Article> = Vec::new();
    for url in urls {
        fetch_articles(url, &mut articles);
    }

    for article in articles.iter().rev() {
        println!("{article}");
    }
}
