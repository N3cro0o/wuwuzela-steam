use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Review {
    pub response: ReviewMessage,
}

impl Review {
    pub fn get_cursor(&self) -> String {
        self.response.cursor.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewMessage {
    query_summary: ReviewSummary,
    reviews: Vec<ReviewBody>,
    cursor: String,
    day_range_used: usize,
    total_matching: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewSummary {
    num_reviews: usize,
    review_score: Option<usize>,
    review_score_desc: Option<String>,
    total_positive: Option<usize>,
    total_negative: Option<usize>,
    total_reviews: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewBody {
    #[serde(rename = "recommendationid")]
    id: String,
    author: Author,
    language: String,
    review: String,
    #[serde(rename = "timestamp_created")]
    created: usize,
    #[serde(rename = "timestamp_updated")]
    updated: usize,
    voted_up: bool,
    votes_up: usize,
    votes_funny: usize,
    weighted_vote_score: f64,
    comment_count: usize,
    steam_purchase: bool,
    received_for_free: bool,
    written_during_early_access: bool,
    dev_response: Option<String>,
    timestamp_dev: Option<usize>,
    primary_steam_deck: Option<bool>,
    refunded: bool,
    hardware: Option<Hardware>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    steamid: String,
    num_reviews: usize,
    playtime_forever: usize,
    playtime_last_two_weeks: usize,
    playtime_at_review: usize,
    deck_playime_at_review: Option<usize>,
    last_played: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hardware {
    os: String,
    cpu_name: String,
    adapter_description: String,
    system_ram: String,
    vram_size: usize,
}
