use std::io;
use std::path::PathBuf;
use std::fs;
use std::error::Error;
use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use reqwest::header::USER_AGENT;
use reqwest::blocking::{Client, Response};

mod steam;

const REVIEW_ADDRESS: &str = "https://api.steampowered.com/IUserReviewsService/GetAppReviews/v1";
const REVIEW_ID: &str = "appid";
const REVIEW_DAY_RANGE: &str = "day_range";
const REVIEW_COUNT: &str = "num_per_page";

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
    println!("3. Clean selected review files,");

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
            for _ in 0..count {
                let response = get_reviews(id, cursor, options.clone())?;
                if !response.status().is_success() {
                    eprintln!("We fucked up");
                    return Ok(0);
                }
                let mut body = response.text()?;
                let response_struct: steam::Review = serde_json::from_str(&body)?;
                cursor = Some(response_struct.get_cursor());
                body = body + "\n";
                file.write_all(body.as_bytes())?;
            }
            Ok(0)
        }

        3 => {
            clean_review_files()?;
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

fn clean_review_files() -> Result<(), Box<dyn Error>> {
    let mut files = Vec::new();
    for entry in fs::read_dir("data")? {
        let entry = entry?;
        let path = entry.path();
        let is_json = path
            .extension()
            .is_some_and(|extension| extension == "json");
        let is_cleaned = path.file_stem().is_some_and(|stem| {
            let stem = stem.to_string_lossy();
            stem.ends_with("_cleaned") || stem.contains("_cleaned (")
        });
        if entry.file_type()?.is_file() && is_json && !is_cleaned {
            files.push(path);
        }
    }
    files.sort();

    if files.is_empty() {
        println!("No review JSON files found in the data folder.");
        return Ok(());
    }

    println!("Available review files:");
    for (index, path) in files.iter().enumerate() {
        println!(
            "{}. {}",
            index + 1,
            path.file_name().unwrap().to_string_lossy()
        );
    }
    println!("Enter file numbers separated by commas, 'all' for every file, or 0 to cancel:");
    let mut selection = String::new();
    get_input(&mut selection)?;
    let selection = selection.trim();

    if selection.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No files selected").into());
    }
    if selection == "0" {
        return Ok(());
    }

    let mut selected_indices: Vec<usize> = if selection.eq_ignore_ascii_case("all") {
        (0..files.len()).collect()
    } else {
        selection
            .split(',')
            .map(|part| {
                let index = part.trim().parse::<usize>()?;
                if index == 0 || index > files.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "File number is out of range",
                    )
                    .into());
                }
                Ok(index - 1)
            })
            .collect::<Result<_, Box<dyn Error>>>()?
    };
    selected_indices.sort_unstable();
    selected_indices.dedup();

    println!("Cleaning options:");
    println!("1. Remove multi-line ASCII art blocks");
    println!("2. Remove duplicate reviews (by review ID)");
    println!("3. Remove reviews below a minimum playtime (hours)");
    println!("Enter option numbers separated by commas, or 0 to cancel:");
    let mut cleaning_selection = String::new();
    get_input(&mut cleaning_selection)?;
    let cleaning_selection = cleaning_selection.trim();
    if cleaning_selection.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No cleaning option selected").into());
    }
    if cleaning_selection == "0" {
        return Ok(());
    }

    let mut clean_ascii_art = false;
    let mut remove_duplicates = false;
    let mut minimum_playtime_hours = None;
    for option in cleaning_selection.split(',').map(str::trim) {
        match option {
            "1" => clean_ascii_art = true,
            "2" => remove_duplicates = true,
            "3" => {
                println!("Minimum playtime in hours:");
                let mut hours = String::new();
                get_input(&mut hours)?;
                minimum_playtime_hours = Some(hours.trim().parse::<u64>()?);
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown cleaning option: {option}"),
                )
                .into());
            }
        }
    }
    let cleaning_options = CleaningOptions {
        remove_ascii_art: clean_ascii_art,
        remove_duplicates,
        minimum_playtime_hours,
    };

    for index in selected_indices {
        let source = &files[index];
        let content = fs::read_to_string(source)?;
        let (cleaned, stats) = clean_review_data(&content, &cleaning_options).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Could not clean {}: {error}", source.display()),
            )
        })?;
        let stem = source.file_stem().unwrap().to_string_lossy();
        let mut suffix = 0;
        let output = loop {
            let name = if suffix == 0 {
                format!("{stem}_cleaned.json")
            } else {
                format!("{stem}_cleaned ({suffix}).json")
            };
            let output = source.with_file_name(name);
            if !output.exists() {
                break output;
            }
            suffix += 1;
        };
        fs::write(&output, cleaned)?;
        println!(
            "Saved {} ({} duplicate reviews removed, {} below the playtime threshold removed, {} reviews had ASCII art removed, {} reviews had no playtime data).",
            output.file_name().unwrap().to_string_lossy(),
            stats.duplicates_removed,
            stats.below_playtime_removed,
            stats.ascii_art_cleaned,
            stats.playtime_missing
        );
    }

    Ok(())
}

#[derive(Default)]
struct CleaningStats {
    duplicates_removed: usize,
    below_playtime_removed: usize,
    ascii_art_cleaned: usize,
    playtime_missing: usize,
}

struct CleaningOptions {
    remove_ascii_art: bool,
    remove_duplicates: bool,
    minimum_playtime_hours: Option<u64>,
}

fn clean_review_data(
    content: &str,
    options: &CleaningOptions,
) -> Result<(String, CleaningStats), Box<dyn Error>> {
    if let Ok(mut value) = serde_json::from_str::<Value>(content) {
        let mut stats = CleaningStats::default();
        clean_reviews_from_page(&mut value, options, &mut stats)?;
        let mut output = serde_json::to_string_pretty(&value)?;
        output.push('\n');
        return Ok((output, stats));
    }

    let mut output = String::new();
    let mut stats = CleaningStats::default();
    let mut seen_review_ids = HashSet::new();
    for (line_number, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut value: Value = serde_json::from_str(line).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid JSON on line {}: {error}", line_number + 1),
            )
        })?;
        clean_reviews_from_page_with_seen_ids(
            &mut value,
            options,
            &mut stats,
            &mut seen_review_ids,
        )?;
        output.push_str(&serde_json::to_string(&value)?);
        output.push('\n');
    }
    if output.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file contains no JSON records").into());
    }
    Ok((output, stats))
}

fn clean_reviews_from_page(
    value: &mut Value,
    options: &CleaningOptions,
    stats: &mut CleaningStats,
) -> Result<(), Box<dyn Error>> {
    let mut seen_review_ids = HashSet::new();
    clean_reviews_from_page_with_seen_ids(value, options, stats, &mut seen_review_ids)
}

fn clean_reviews_from_page_with_seen_ids(
    value: &mut Value,
    options: &CleaningOptions,
    stats: &mut CleaningStats,
    seen_review_ids: &mut HashSet<String>,
) -> Result<(), Box<dyn Error>> {
    let Some(reviews) = value
        .get_mut("response")
        .and_then(|response| response.get_mut("reviews"))
        .and_then(Value::as_array_mut)
    else {
        return Ok(());
    };

    let mut kept_reviews = Vec::with_capacity(reviews.len());
    for mut review in std::mem::take(reviews) {
        if options.remove_duplicates {
            if let Some(id) = review.get("recommendationid").and_then(Value::as_str) {
                if !seen_review_ids.insert(id.to_owned()) {
                    stats.duplicates_removed += 1;
                    continue;
                }
            }
        }

        if let Some(minimum_hours) = options.minimum_playtime_hours {
            let minimum_minutes = minimum_hours.checked_mul(60).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Playtime threshold is too large")
            })?;
            let playtime_minutes = review
                .get("author")
                .and_then(|author| author.get("playtime_at_review"))
                .and_then(Value::as_u64);
            match playtime_minutes {
                Some(minutes) if minutes < minimum_minutes => {
                    stats.below_playtime_removed += 1;
                    continue;
                }
                Some(_) => {}
                None => stats.playtime_missing += 1,
            }
        }

        if options.remove_ascii_art {
            if let Some(text) = review.get("review").and_then(Value::as_str) {
                let cleaned = remove_ascii_art(text);
                if cleaned != text {
                    review["review"] = Value::String(cleaned);
                    stats.ascii_art_cleaned += 1;
                }
            }
        }
        kept_reviews.push(review);
    }

    *reviews = kept_reviews;
    Ok(())
}

fn remove_ascii_art(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut output = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if !is_ascii_art_line(lines[index]) {
            output.push(lines[index]);
            index += 1;
            continue;
        }

        let start = index;
        let mut art_lines = 0;
        while index < lines.len() {
            if is_ascii_art_line(lines[index]) {
                art_lines += 1;
                index += 1;
            } else if lines[index].trim().is_empty()
                && index + 1 < lines.len()
                && is_ascii_art_line(lines[index + 1])
            {
                index += 1;
            } else {
                break;
            }
        }

        if art_lines >= 2 {
            continue;
        }
        output.extend_from_slice(&lines[start..index]);
    }

    output.join("\n")
}

fn is_ascii_art_line(line: &str) -> bool {
    let line = line.trim();
    let characters: Vec<char> = line.chars().collect();
    let art_symbols = characters
        .iter()
        .filter(|character| {
            matches!(
                character,
                '/' | '\\' | '|' | '_' | '[' | ']' | '{' | '}' | '<' | '>' | '(' | ')' | '^'
            )
        })
        .count();
    let alphanumeric = characters
        .iter()
        .filter(|character| character.is_ascii_alphanumeric())
        .count();
    let non_whitespace = characters
        .iter()
        .filter(|character| !character.is_whitespace())
        .count();

    line.is_ascii()
        && characters.len() >= 3
        && art_symbols >= 2
        && alphanumeric <= 2
        && non_whitespace > 0
        && art_symbols >= alphanumeric
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
            &[(REVIEW_ID, game_id.to_string()), (REVIEW_COUNT, options.reviews_per_page.to_string()),
            ("cursor", s), (REVIEW_DAY_RANGE, options.day_range.to_string())]);
    } else {
        url = reqwest::Url::parse_with_params(REVIEW_ADDRESS,
            &[(REVIEW_ID, game_id.to_string()), (REVIEW_COUNT, options.reviews_per_page.to_string()),
            ("cursor", "*".to_string()), (REVIEW_DAY_RANGE, options.day_range.to_string())]);
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
    let options = init()?;
    main_loop(options)?;
    Ok(())
}
