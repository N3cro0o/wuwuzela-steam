import json
import sqlite3
from pathlib import Path

PROJECT_DIR = Path(__file__).resolve().parent
DATA_DIR = PROJECT_DIR / "data"
DB_PATH = PROJECT_DIR / "steam.db"
GAMES_FILE = DATA_DIR / "games.json"

REVIEW_FIELDS = [
    "recommendationid",
    "language",
    "review",
    "timestamp_created",
    "timestamp_updated",
    "voted_up",
    "votes_up",
    "votes_funny",
    "weighted_vote_score",
    "comment_count",
    "steam_purchase",
    "received_for_free",
    "refunded",
    "written_during_early_access",
]

AUTHOR_FIELDS = [
    "steamid",
    "num_reviews",
    "playtime_forever",
    "playtime_last_two_weeks",
    "playtime_at_review",
    "last_played",
]

SCHEMA = """
CREATE TABLE IF NOT EXISTS games (
    app_id        INTEGER PRIMARY KEY,
    name          TEXT NOT NULL,
    total_reviews INTEGER
);

CREATE TABLE IF NOT EXISTS reviews (
    recommendationid            INTEGER PRIMARY KEY,
    app_id                      INTEGER NOT NULL REFERENCES games(app_id),
    language                    TEXT,
    review                      TEXT,
    timestamp_created           INTEGER,
    timestamp_updated           INTEGER,
    voted_up                    BOOLEAN,
    votes_up                    INTEGER,
    votes_funny                 INTEGER,
    weighted_vote_score         REAL,
    comment_count               INTEGER,
    steam_purchase              BOOLEAN,
    received_for_free           BOOLEAN,
    refunded                    BOOLEAN,
    written_during_early_access BOOLEAN,
    steamid                     INTEGER,
    num_reviews                 INTEGER,
    playtime_forever            INTEGER,
    playtime_last_two_weeks     INTEGER,
    playtime_at_review          INTEGER,
    last_played                 INTEGER
);

CREATE INDEX IF NOT EXISTS idx_reviews_app_date
    ON reviews (app_id, timestamp_created);

DROP VIEW IF EXISTS reviews_view;
CREATE VIEW reviews_view AS
SELECT
    recommendationid,
    app_id,
    language,
    review,
    datetime(timestamp_created, 'unixepoch', 'localtime') AS timestamp_created,
    datetime(timestamp_updated, 'unixepoch', 'localtime') AS timestamp_updated,
    voted_up,
    votes_up,
    votes_funny,
    weighted_vote_score,
    comment_count,
    steam_purchase,
    received_for_free,
    refunded,
    written_during_early_access,
    steamid,
    num_reviews,
    playtime_forever,
    playtime_last_two_weeks,
    playtime_at_review,
    datetime(last_played, 'unixepoch', 'localtime') AS last_played
FROM reviews;
"""

ALL_COLUMNS = ["app_id"] + REVIEW_FIELDS + AUTHOR_FIELDS
UPSERT_REVIEW = (
    f"INSERT INTO reviews ({', '.join(ALL_COLUMNS)}) "
    f"VALUES ({', '.join('?' for _ in ALL_COLUMNS)}) "
    f"ON CONFLICT(recommendationid) DO UPDATE SET "
    + ", ".join(f"{c} = excluded.{c}" for c in ALL_COLUMNS if c != "recommendationid")
)

UPSERT_GAME = (
    "INSERT INTO games (app_id, name, total_reviews) VALUES (?, ?, ?) "
    "ON CONFLICT(app_id) DO UPDATE SET "
    "name = excluded.name, "
    "total_reviews = COALESCE(excluded.total_reviews, games.total_reviews)"
)


def load_game_names():
    if not GAMES_FILE.exists():
        print(f"WARNING: {GAMES_FILE.name} not found, game names will be app ids.")
        return {}
    with open(GAMES_FILE, encoding="utf-8") as f:
        return {int(app_id): name for app_id, name in json.load(f).items()}


def read_reviews(path):
    with open(path, encoding="utf-8") as f:
        for line_number, line in enumerate(f, start=1):
            if not line.strip():
                continue
            try:
                response = json.loads(line)["response"]
            except (json.JSONDecodeError, KeyError) as error:
                print(f"WARNING: {path.name} line {line_number} skipped ({error}).")
                continue
            total = response.get("query_summary", {}).get("total_reviews")
            yield response.get("reviews", []), total


def review_to_row(app_id, review):
    author = review.get("author", {})
    data = {field: review.get(field) for field in REVIEW_FIELDS}
    data.update({field: author.get(field) for field in AUTHOR_FIELDS})
    for field in ("recommendationid", "steamid"):
        if data[field] is not None:
            data[field] = int(data[field])
    return (app_id,) + tuple(data[field] for field in REVIEW_FIELDS + AUTHOR_FIELDS)


def main():
    files = sorted(DATA_DIR.glob("*_cleaned.json"))
    if not files:
        print(f"No *_cleaned.json files in {DATA_DIR}. Nothing to do.")
        return

    names = load_game_names()
    connection = sqlite3.connect(DB_PATH)
    connection.execute("PRAGMA foreign_keys = ON")
    connection.executescript(SCHEMA)

    for path in files:
        app_id_text = path.name.removesuffix("_cleaned.json")
        if not app_id_text.isdigit():
            print(f"WARNING: {path.name} skipped (file name does not start with an app id).")
            continue
        app_id = int(app_id_text)

        if app_id not in names:
            print(f"WARNING: {app_id} is missing in {GAMES_FILE.name}, using the app id as name.")
        name = names.get(app_id, str(app_id))

        rows = []
        total_reviews = None
        for reviews, total in read_reviews(path):
            if total is not None and total_reviews is None:
                total_reviews = total
            rows += [review_to_row(app_id, r) for r in reviews if r.get("recommendationid")]

        before = connection.execute(
            "SELECT COUNT(*) FROM reviews WHERE app_id = ?", (app_id,)
        ).fetchone()[0]

        with connection:
            connection.execute(UPSERT_GAME, (app_id, name, total_reviews))
            connection.executemany(UPSERT_REVIEW, rows)

        after = connection.execute(
            "SELECT COUNT(*) FROM reviews WHERE app_id = ?", (app_id,)
        ).fetchone()[0]
        print(f"{name} ({app_id}): {len(rows)} reviews in file, "
              f"{after - before} new, {after} in database.")

    games_count, reviews_count = connection.execute(
        "SELECT (SELECT COUNT(*) FROM games), (SELECT COUNT(*) FROM reviews)"
    ).fetchone()
    connection.close()
    print(f"Done: {games_count} games, {reviews_count} reviews in {DB_PATH.name}.")


if __name__ == "__main__":
    main()
