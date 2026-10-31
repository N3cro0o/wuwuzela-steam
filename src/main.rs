use std::io;
use std::path::PathBuf;
use std::fs;
use std::error::Error;
use serde::{Serialize, Deserialize};

use reqwest::header::USER_AGENT;
use reqwest::blocking::{Client, Response};

mod steam;

const REVIEW_ADDRESS: &str = "https://api.steampowered.com/IUserReviewsService/GetAppReviews/v1";
const REVIEW_ID: &str = "appid";
const REVIEW_DAY_RANGE: &str = "day_range";
const REVIEW_COUNT: &str = "num_per_page=";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FetcherOptions {
    pub reviews_per_page: u32,
    pub day_range: u32,
}

impl FetcherOptions {
    pub fn new() -> Self {
        FetcherOptions {
            reviews_per_page: 30,
            day_range: 0,
        }
    }
}

fn main_loop(opt: FetcherOptions) -> Result<(), Box<dyn Error>>{
    let mut options = opt;
    let mut input_str = String::new();
    loop {
        print_menu();
        get_input(&mut input_str)?;
        let o = handle_input(&input_str, &mut options)?;
        if o == 1 {
            break;
        }
        input_str.clear();
    }
    Ok(())
}

fn print_menu() {
    println!("================================\n");
    println!("Hello in Steam ripper? Steam fetcher? Steam review yoinker? Whatever. Select opion below and let's proceed:");
    println!("1. Fetch one page of reviews for seleced game,");
    println!("2. Fetch more pages of reviews for seleced game (not implemented),");

    println!("9. Change options (not implemented),");
    println!("10. Test system.\n");
    println!("Write input below and press enter. To close type anything that isn't listed above:");
}

fn get_input(output: &mut String) -> Result<(), Box<dyn Error>> {
    io::stdin().read_line(output)?;
    println!("Echo: {}", output);
    Ok(())
}

fn handle_input(output: &str, options: &mut FetcherOptions) -> Result<i32, Box<dyn Error>> {
    use std::io::Write;
    let output_i32: i32 = output.trim().parse::<i32>()?;
    match output_i32 {
        1 => {
            let id = get_steam_game_id()?;
            let response = get_reviews(id, None, options.clone())?;
            if !response.status().is_success() {
                eprintln!("We fucked up");
                return Ok(0);
            }
            let body = response.text()?;
            let mut file = create_file(id)?;
            file.write_all(body.as_bytes())?;
            Ok(0)
        }

        2 => {
            let id = get_steam_game_id()?;
            let mut file = create_file(id)?;
            println!("How many pages do you need laddie?");
            let mut count_str = String::new();
            get_input(&mut count_str)?;
            let count = count_str.trim().parse::<u32>()?;
            let mut cursor = None;
            for i in 0..count {
                let response = get_reviews(id, cursor, options.clone())?;
                if !response.status().is_success() {
                    eprintln!("We fucked up");
                    return Ok(0);
                }
                let body = response.text()?;
                let response_struct: steam::Review = serde_json::from_str(&body)?;
                cursor = Some(response_struct.get_cursor());
                file.write_all(body.as_bytes())?;
            }
            Ok(0)
        }

        9 => {
            Ok(0)
        }

        10 => {
            let id = 3191050;
            let response = get_reviews(id, None, options.clone())?;
            if !response.status().is_success() {
                eprintln!("We fucked up");
                return Ok(0);
            }
            println!("------------------------------------------------------");
            let body = response.text()?;
            println!("{body}");
            let response_struct: steam::Review = serde_json::from_str(&body)?;
            println!("{:?}", response_struct);
            println!("------------------------------------------------------");
            let response = get_reviews(id, Some(response_struct.get_cursor()), options.clone())?;
            if !response.status().is_success() {
                eprintln!("We fucked up");
                return Ok(0);
            }
            let body = response.text()?;
            println!("{body}");
            let response_struct: steam::Review = serde_json::from_str(&body)?;
            println!("{:?}", response_struct);
            println!("------------------------------------------------------");
            Ok(0)
        }

        _ => Ok(1)
    }
}

fn create_file(id: usize) -> std::io::Result<fs::File> {
    let mut path = PathBuf::from("data");
    let mut count = 0;
    for entry in path.as_path().read_dir()? {
        let e = entry?;
        let os_name = e.file_name();
        let name = os_name.into_string().unwrap();
        if name.contains(&id.to_string()){
            count += 1;
        }
    }
    let file_name;
    if count > 0 {
        file_name = format!("{} ({}).json", id, count);
    }
    else {
        file_name = format!("{}.json", id);
    }
    path.push(file_name);
    let file = fs::OpenOptions::new().read(true).write(true).create(true).open(path)?;
    Ok(file)
}

fn get_steam_game_id() -> Result<usize, Box<dyn Error>> {
    println!("Input steam game ID (or steam item ID, the same thing)");
    let mut string = String::new();
    get_input(&mut string)?;
    let id = string.trim().parse::<usize>()?;
    Ok(id)
}

fn prepare_url(game_id: usize, cursor: Option<String>, options: FetcherOptions) -> reqwest::Url {
    let url;
    if let Some(s) = cursor {
        url = reqwest::Url::parse_with_params(REVIEW_ADDRESS,
            &[(REVIEW_ID, game_id.to_string()), (REVIEW_COUNT, options.reviews_per_page.to_string()), ("cursor", s)]);
    }
    else {
        url = reqwest::Url::parse_with_params(REVIEW_ADDRESS,
            &[(REVIEW_ID, game_id.to_string()), (REVIEW_COUNT, options.reviews_per_page.to_string())]);
    }
    url.unwrap()
}

fn get_reviews(game_id: usize, cursor: Option<String>, options: FetcherOptions) -> reqwest::Result<Response> {
    let url = prepare_url(game_id, cursor, options);
    println!("URL: {url}");
    let client = Client::new();
    let response = client.get(url)
        .header(USER_AGENT, "PWR - data visualisation project.")
        .send();
    response
}

fn init() -> Result<FetcherOptions, Box<dyn Error>> {
    let path = PathBuf::from("data");
    if !path.as_path().exists() {
        fs::create_dir("data")?;
    }
    let path = PathBuf::from("options.json");
    let options;
    if path.as_path().exists() {
        use std::io::Read;
        let mut file = fs::OpenOptions::new().read(true).open(&path)?;
        let mut json = String::new();
        let _ = file.read_to_string(&mut json)?;
        options = serde_json::from_str(&json)?;
    }
    else {
        use std::io::Write;
        options = FetcherOptions::new();
        let mut file = fs::OpenOptions::new().write(true).create(true).open(&path)?;
        file.write_all(serde_json::to_string(&options)?.as_bytes())?;
    }
    Ok(options)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut options = init()?;
    main_loop(options)?;
    Ok(())
}
