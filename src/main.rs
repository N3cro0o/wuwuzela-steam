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
#[serde(default)]
struct FetcherOptions {
    pub reviews_per_page: u32,
    pub day_range: u32,
    pub remove_ascii_art: bool,
    pub remove_duplicates: bool,
    pub minimum_playtime_hours: Option<u64>,
}

impl FetcherOptions {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for FetcherOptions {
    fn default() -> Self {
        FetcherOptions {
            reviews_per_page: 30,
            day_range: 0,
            remove_ascii_art: false,
            remove_duplicates: false,
            minimum_playtime_hours: None,
        }
    }
}

#[derive(Default)]
struct CleaningStats {
    duplicates_removed: usize,
    below_playtime_removed: usize,
    ascii_art_cleaned: usize,
    playtime_missing: usize,
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

    println!("9. Change options,");
    println!("10. Test system.\n");
    println!("Write input below and press enter. To close type anything that isn't listed above:");
}

fn get_input(output: &mut String) -> Result<(), Box<dyn Error>> {
    io::stdin().read_line(output)?;
    println!("Echo: {}", output);
    Ok(())
}

/// Pobiera podaną liczbę stron recenzji i zapisuje je do pliku w folderze data.
fn download_pages(id: usize, count: u32, options: &FetcherOptions) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    // Tworzy plik, do którego zapiszemy pobrane strony.
    let (path, mut file) = create_file(id)?;

    // Kursor mówi Steamowi, którą stronę pobrać następną.
    let mut cursor = None;

    // Zlicza wszystkie zapisane recenzje.
    let mut total_reviews = 0;

    for page_number in 1..=count {
        let response = get_reviews(id, cursor, options.clone())?;

        // Przy błędzie HTTP pokazujemy kod i kończymy pobieranie.
        if !response.status().is_success() {
            eprintln!("We fucked up. HTTP error code: {}", response.status().as_u16());
            return Ok(());
        }

        let body = response.text()?;
        let mut response_struct: steam::Review = serde_json::from_str(&body)?;
        let page_review_count = response_struct.get_review_count();

        // Pusta strona oznacza, że nie ma już recenzji.
        if page_review_count == 0 {
            println!("Strona {page_number} jest pusta. Kończę pobieranie.");
            break;
        }

        // Pomija recenzje bez tekstu i liczy, ile zapisujemy.
        let empty_review_count = response_struct.remove_empty_reviews();
        let saved_review_count = page_review_count - empty_review_count;
        total_reviews += saved_review_count;

        cursor = Some(response_struct.get_cursor());

        // Zapisuje oczyszczoną stronę jako jedną linię pliku.
        let cleaned_body = serde_json::to_string(&response_struct)?;
        file.write_all(format!("{cleaned_body}\n").as_bytes())?;

        println!(
            "Strona {page_number}: zapisano {saved_review_count} recenzji, pominięto {empty_review_count} pustych. Łącznie zapisano: {total_reviews}."
        );
    }

    // Zamyka plik przed dodatkowym czyszczeniem.
    drop(file);
    clean_review_file(&path, options)?;
    Ok(())
}

/// Tryb CLI: steam-fetcher [app_id] [ile stron]
fn run_cli(args: &[String], options: &FetcherOptions) -> Result<(), Box<dyn Error>> {
    // Potrzebujemy dokładnie 2 argumentów (plus nazwa programu na początku).
    if args.len() != 3 {
        eprintln!("Użycie: steam-fetcher [app_id] [ile stron]");
        return Ok(());
    }

    // args[0] to nazwa programu, args[1] to app_id, args[2] to liczba stron.
    let id = match args[1].parse::<usize>() {
        Ok(number) => number,
        Err(_) => {
            eprintln!("app_id musi być liczbą, a dostałem: {}", args[1]);
            return Ok(());
        }
    };

    let count = match args[2].parse::<u32>() {
        Ok(number) => number,
        Err(_) => {
            eprintln!("Liczba stron musi być liczbą, a dostałem: {}", args[2]);
            return Ok(());
        }
    };

    // Używamy tej samej funkcji co opcja 2 w menu.
    download_pages(id, count, options)
}

// Pobiera dwie strony recenzji i wypisuje je w terminalu.
fn test_reviews(id: usize, options: &FetcherOptions) -> Result<(), Box<dyn Error>> {
    // Pobiera pierwszą stronę.
    let response = get_reviews(id, None, options.clone())?;

    if !response.status().is_success() {
        eprintln!("We fucked up. HTTP error code: {}", response.status().as_u16());
        return Ok(());
    }

    println!("------------------------------------------------------");
    let body = response.text()?;
    println!("{body}");

    // Odczytuje kursor potrzebny do pobrania drugiej strony.
    let response_struct: steam::Review = serde_json::from_str(&body)?;
    println!("{:?}", response_struct);
    println!("------------------------------------------------------");

    // Pobiera drugą stronę, używając kursora z pierwszej.
    let response = get_reviews(id, Some(response_struct.get_cursor()), options.clone())?;

    if !response.status().is_success() {
        eprintln!("We fucked up. HTTP error code: {}", response.status().as_u16());
        return Ok(());
    }

    let body = response.text()?;
    println!("{body}");

    let response_struct: steam::Review = serde_json::from_str(&body)?;
    println!("{:?}", response_struct);
    println!("------------------------------------------------------");

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
                eprintln!("We fucked up. HTTP error code: {}", response.status().as_u16());
                return Ok(0);
            }
            let body = response.text()?;
            let mut response_struct: steam::Review = serde_json::from_str(&body)?;
            let page_review_count = response_struct.get_review_count();
            let empty_review_count = response_struct.remove_empty_reviews();
            let saved_review_count = page_review_count - empty_review_count;
            let cleaned_body = serde_json::to_string(&response_struct)?;

            let (path, mut file) = create_file(id)?;
            file.write_all(cleaned_body.as_bytes())?;
            drop(file);
            clean_review_file(&path, options)?;
            println!(
                "Zapisano {saved_review_count} recenzji, pominięto {empty_review_count} pustych."
            );
            Ok(0)
        }

        2 => {
            let id = get_steam_game_id()?;
            println!("How many pages do you need laddie?");
            let mut count_str = String::new();
            get_input(&mut count_str)?;
            let count = count_str.trim().parse::<u32>()?;

            // Całe pobieranie jest w osobnej funkcji, z której skorzysta też tryb CLI.
            download_pages(id, count, options)?;
            Ok(0)
        }

        9 => {
            configure_options(options)?;
            Ok(0)
        }

        10 => {
            // Opcja 10 nadal testuje tę samą grę co wcześniej.
            let id = 3191050;
            test_reviews(id, options)?;
            Ok(0)
        }

        _ => Ok(1)
    }
}

fn configure_options(options: &mut FetcherOptions) -> Result<(), Box<dyn Error>> {
    println!("Options:");
    println!("1. Change reviews per page (currently {})", options.reviews_per_page);
    println!("2. Change day range (currently {})", options.day_range);
    println!("3. Configure automatic cleaning filters");
    println!("Enter an option number, or 0 to cancel:");
    let mut selection = String::new();
    get_input(&mut selection)?;

    match selection.trim().parse::<u32>()? {
        0 => Ok(()),
        1 => {
            println!("Reviews per page (must be greater than 0):");
            let mut value = String::new();
            get_input(&mut value)?;
            let reviews_per_page = value.trim().parse::<u32>()?;
            if reviews_per_page == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Reviews per page must be greater than 0",
                )
                .into());
            }
            options.reviews_per_page = reviews_per_page;
            save_options(options)?;
            println!("Reviews per page saved to options.json.");
            Ok(())
        }
        2 => {
            println!("Day range in days (0 means no day-range limit):");
            let mut value = String::new();
            get_input(&mut value)?;
            options.day_range = value.trim().parse::<u32>()?;
            save_options(options)?;
            println!("Day range saved to options.json.");
            Ok(())
        }
        3 => configure_cleaning_options(options),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unknown option").into()),
    }
}

fn configure_cleaning_options(options: &mut FetcherOptions) -> Result<(), Box<dyn Error>> {
    println!("Cleaning filters:");
    println!("1. Remove multi-line ASCII art blocks");
    println!("2. Remove duplicate reviews (by review ID)");
    println!("3. Remove reviews below a minimum playtime (hours)");
    println!("Enter option numbers separated by commas, 'none' to disable all, or 0 to cancel:");
    let mut selection = String::new();
    get_input(&mut selection)?;
    let selection = selection.trim();

    if selection == "0" {
        return Ok(());
    }
    if selection.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No cleaning option selected").into());
    }

    let mut remove_ascii_art = false;
    let mut remove_duplicates = false;
    let mut minimum_playtime_hours = None;
    if !selection.eq_ignore_ascii_case("none") {
        for option in selection.split(',').map(str::trim) {
            match option {
                "1" => remove_ascii_art = true,
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
    }

    options.remove_ascii_art = remove_ascii_art;
    options.remove_duplicates = remove_duplicates;
    options.minimum_playtime_hours = minimum_playtime_hours;
    save_options(options)?;
    println!("Cleaning options saved to options.json.");
    Ok(())
}

fn clean_review_file(source: &PathBuf, options: &FetcherOptions) -> Result<(), Box<dyn Error>> {
    if !options.remove_ascii_art
        && !options.remove_duplicates
        && options.minimum_playtime_hours.is_none()
    {
        println!("No cleaning filters are enabled; saved the raw file only.");
        return Ok(());
    }

    let content = fs::read_to_string(source)?;
    let (cleaned, stats) = clean_review_data(&content, options).map_err(|error| {
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
    Ok(())
}

fn clean_review_data(
    content: &str,
    options: &FetcherOptions,
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
    options: &FetcherOptions,
    stats: &mut CleaningStats,
) -> Result<(), Box<dyn Error>> {
    let mut seen_review_ids = HashSet::new();
    clean_reviews_from_page_with_seen_ids(value, options, stats, &mut seen_review_ids)
}

fn clean_reviews_from_page_with_seen_ids(
    value: &mut Value,
    options: &FetcherOptions,
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

fn create_file(id: usize) -> std::io::Result<(PathBuf, fs::File)> {
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
    let file = fs::OpenOptions::new().read(true).write(true).create(true).open(&path)?;
    Ok((path, file))
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

fn save_options(options: &FetcherOptions) -> Result<(), Box<dyn Error>> {
    fs::write("options.json", serde_json::to_string_pretty(options)?)?;
    Ok(())
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
    } else {
        options = FetcherOptions::new();
    }
    save_options(&options)?;
    Ok(options)
}

fn main() -> Result<(), Box<dyn Error>> {
    // Wczytuje ustawienia, tak jak wcześniej.
    let options = init()?;

    // Zbiera nazwę programu i argumenty wpisane w terminalu.
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Są argumenty: uruchamiamy tryb CLI.
        run_cli(&args, &options)?;
    } else {
        // Nie ma argumentów: uruchamiamy dotychczasowe menu.
        main_loop(options)?;
    }

    Ok(())
}
