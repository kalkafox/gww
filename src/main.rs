use std::{io::stdout, net::Ipv4Addr, process::Command, thread, time::Duration};

use clap::Parser;
use colored::Colorize;
use crossterm::{
    cursor::{position, MoveDown, MoveUp},
    event::{Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use dialoguer::FuzzySelect;
use futures::{select, FutureExt, StreamExt};
use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    Response,
};
use serde::{Deserialize, Serialize};

pub type Search = Vec<SearchElement>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum SearchElement {
    String(String),
    StringArray(Vec<String>),
}

/// Guild Wars Wiki
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// GW2 search term
    #[arg(default_value = "")]
    search_term: String,

    /// Skip the selection when using a search term.
    #[arg(short, long)]
    skip_selection: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Check if we have an internet connection
    reqwest::get("https://google.com")
        .await?
        .error_for_status()?;

    let args = Args::parse();

    let mut reader = EventStream::new();

    let mut terms: Vec<String> = vec![];
    let mut search_input = args.search_term; // To store input

    let mut selection = 0;

    // really dumb and hacky but im tired and idc
    let mut first_run = false;

    let mut headers = HeaderMap::new();

    headers.append(USER_AGENT, HeaderValue::from_static("gww (made by Kalka)"));

    let client = reqwest::Client::new();
    let mut last_count = terms.len();

    let mut term_urls = vec![];

    let mut url = String::new();

    if !search_input.is_empty() {
        let res = client.get(format!("https://wiki.guildwars2.com/api.php?action=opensearch&format=json&formatversion=2&search={search_input}&namespace=0&limit=10")).headers(headers.clone()).send().await?;
        let json = res.json::<Search>().await?;

        match &json[1] {
            SearchElement::StringArray(w) => {
                if !w.is_empty() {
                    terms.extend(w.to_owned())
                }
            }
            SearchElement::String(w) => {
                println!("{}", w);
            }
        }

        match &json[3] {
            SearchElement::StringArray(w) => term_urls.extend(w.to_owned()),
            _ => {}
        }

        if args.skip_selection {
            open_url(term_urls[0].to_owned())?;
        } else {
            let selection = FuzzySelect::new()
                .with_prompt("Make a choice:")
                .items(&terms)
                .interact()?;

            open_url(term_urls[selection].to_owned())?;
        }

        return Ok(());
    }

    enable_raw_mode()?;

    loop {
        let mut event = reader.next().fuse();

        select! {
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(Event::Key(key_event))) => {
                        // Clear previous output
                        let mut stdout = stdout();

                        let mut count = terms.len();

                        // Clear the number of lines equal to the current number of terms
                        match key_event.code {
                            // Handle normal character input
                            KeyCode::Char(c) => {
                                search_input.push(c);

                                if first_run {
                                    execute!(stdout, MoveUp(u16::try_from(if count > 0 {count+1} else {1}).unwrap()), Clear(ClearType::CurrentLine))?;
                                }

                                println!("{search_input}\r");

                                let res = client.get(format!("https://wiki.guildwars2.com/api.php?action=opensearch&format=json&formatversion=2&search={search_input}&namespace=0&limit=10")).headers(headers.clone()).send().await?;
                                let json = res.json::<Search>().await?;
                                let search = json.clone();





                                //println!("{search:?}");

                                match &search[1] {
                                    SearchElement::StringArray(w) => {
                                        terms.clear();
                                        if !w.is_empty() {
                                            terms.extend(w.to_owned())
                                        }
                                    }
                                    SearchElement::String(w) => {
                                        println!("{}", w);
                                    }
                                }

                                match &search[3] {
                                    SearchElement::StringArray(w) => {
                                        term_urls.clear();
                                        term_urls.extend(w.to_owned())
                                    }
                                    _ => {}
                                }

                                if first_run {
                                    execute!(stdout, MoveDown(u16::try_from(if count > 0 {count} else {1}).unwrap()), Clear(ClearType::CurrentLine))?;
                                } else {
                                    execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine))?;
                                }


                                count = terms.len();
                            }

                            KeyCode::Down => {
                                if count == 0 {
                                    continue;
                                }
                                selection = (selection + 1) % count;
                            }

                            KeyCode::Up => {
                                if count == 0 {
                                    continue;
                                }
                                if selection == 0 {
                                    selection = count - 1;
                                } else {
                                    selection -= 1;
                                }
                            }

                            KeyCode::Enter => {

                                if last_count == 0 {
                                    continue;
                                }

                                if first_run {
                                    for k in 0..last_count {
                                        // if k == 0 {
                                        //     execute!(stdout, Clear(ClearType::CurrentLine))?;
                                        // }
                                        execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine))?;
                                    }
                                    execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine))?;
                                }

                                url = term_urls[selection].to_string();

                                break;
                            }

                            // Handle backspace
                            KeyCode::Backspace => {
                                search_input.pop(); // Remove last character from input

                                if first_run && count == 0 {
                                    execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine))?;
                                    print!("{search_input}\r");
                                    execute!(stdout, MoveDown(1))?;
                                    continue;
                                }
                            }

                            // Handle Escape to exit
                            KeyCode::Esc => {
                                break;
                            }

                            _ => {}
                        }

                        if first_run {
                            for _ in 0..last_count {
                                // if k == 0 {
                                //     execute!(stdout, Clear(ClearType::CurrentLine))?;
                                // }
                                execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine))?;
                            }

                            if last_count == 0 {
                                execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine))?;
                            }

                            execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine))?;
                        }

                        println!("{}\r", search_input);

                        // Print the updated search terms
                        for (i, term) in terms.iter().enumerate() {
                            println!("{}\r", if i == selection { term.black().on_white() } else { term.normal() });
                        }

                        first_run = true;
                        last_count = count;
                    }
                    Some(Err(e)) => println!("Error: {:?}\r", e),
                    None => break,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;

    if url.is_empty() {
        return Ok(());
    }

    open_url(url)?;

    Ok(())
}

fn open_url(url: String) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // For Linux
        Command::new("xdg-open").arg(url).spawn()?.wait()?;
    }

    #[cfg(target_os = "macos")]
    {
        // For macOS
        Command::new("open").arg(url).spawn()?.wait()?;
    }

    #[cfg(target_os = "windows")]
    {
        // For Windows
        Command::new("cmd")
            .args(&["/C", "start", url])
            .spawn()?
            .wait()?;
    }

    Ok(())
}
