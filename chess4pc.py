"""
Chess.com 4-Player Chess — Top 10 Leaderboard & Game Archive Fetcher

Retrieves the top 10 players from the Chess.com 4-Player Chess leaderboard,
then fetches their available game archives via the Chess.com Published Data API
(PubAPI). Complies with PubAPI standards:

  - Includes a descriptive User-Agent header with contact info
  - Requests gzip-compressed responses (Accept-Encoding: gzip)
  - Uses If-Modified-Since / If-None-Match caching headers where applicable
  - Respects rate limits: serial requests with backoff on HTTP 429
  - Read-only access to publicly available data

Usage:
    python chess4pc.py [--months N] [--output DIR]

    --months N    How many recent monthly archives to fetch per player (default: 3)
    --output DIR  Directory to write output files into (default: ./chess4pc_output)
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime

import requests

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

PUBAPI_BASE = "https://api.chess.com/pub"
LEADERBOARD_URL = "https://www.chess.com/callback/leaderboard/live/4-player-chess"

USER_AGENT = (
    "Chess4PC-Fetcher/1.0 "
    "(purpose: fetch top-10 4-player-chess leaderboard and game archives; "
    "contact: project-odin@users.noreply.github.com)"
)

HEADERS = {
    "User-Agent": USER_AGENT,
    "Accept-Encoding": "gzip",
    "Accept": "application/json",
}

RATE_LIMIT_WAIT = 10  # seconds to wait on 429
MAX_RETRIES = 3


# ---------------------------------------------------------------------------
# HTTP helpers (PubAPI-compliant)
# ---------------------------------------------------------------------------

def api_get(url, etag=None, last_modified=None):
    """
    GET a URL with PubAPI-compliant headers. Handles 429 rate-limit responses
    with exponential backoff. Returns (response_json, etag, last_modified) or
    (None, None, None) on 304 Not Modified.
    """
    headers = dict(HEADERS)
    if etag:
        headers["If-None-Match"] = etag
    if last_modified:
        headers["If-Modified-Since"] = last_modified

    for attempt in range(1, MAX_RETRIES + 1):
        try:
            resp = requests.get(url, headers=headers, timeout=30)
        except requests.RequestException as exc:
            print(f"  [!] Network error fetching {url}: {exc}")
            if attempt < MAX_RETRIES:
                time.sleep(2 ** attempt)
                continue
            return None, None, None

        if resp.status_code == 200:
            return (
                resp.json(),
                resp.headers.get("ETag"),
                resp.headers.get("Last-Modified"),
            )
        elif resp.status_code == 304:
            return None, etag, last_modified  # data unchanged
        elif resp.status_code == 429:
            wait = RATE_LIMIT_WAIT * attempt
            print(f"  [!] Rate-limited (429). Waiting {wait}s before retry...")
            time.sleep(wait)
            continue
        elif resp.status_code == 404:
            print(f"  [!] Not found (404): {url}")
            return None, None, None
        else:
            print(f"  [!] HTTP {resp.status_code} for {url}")
            if attempt < MAX_RETRIES:
                time.sleep(2 ** attempt)
                continue
            return None, None, None

    return None, None, None


# ---------------------------------------------------------------------------
# Leaderboard
# ---------------------------------------------------------------------------

def fetch_leaderboard(count=10):
    """Fetch the top `count` players from the 4-Player Chess leaderboard."""
    print(f"\n{'='*60}")
    print(f"  Fetching 4-Player Chess Leaderboard (Top {count})")
    print(f"{'='*60}\n")

    data, _, _ = api_get(LEADERBOARD_URL)
    if data is None:
        print("[ERROR] Could not retrieve leaderboard.")
        sys.exit(1)

    leaders = data.get("leaders", [])[:count]

    results = []
    for entry in leaders:
        user = entry.get("user", {})
        info = {
            "rank": entry.get("rank"),
            "username": user.get("username"),
            "rating": entry.get("score"),
            "title": user.get("chess_title", ""),
            "country": user.get("country_name", ""),
            "total_games": entry.get("totalGameCount", 0),
            "wins": entry.get("totalWinCount", 0),
            "losses": entry.get("totalLossCount", 0),
            "draws": entry.get("totalDrawCount", 0),
        }
        results.append(info)
        wl = (
            f"{info['wins']}W / {info['losses']}L / {info['draws']}D"
        )
        title_str = f" ({info['title']})" if info["title"] else ""
        print(
            f"  #{info['rank']:>2}  {info['username']:<20s}{title_str:<6s}  "
            f"Rating: {info['rating']}  |  {wl}  |  {info['country']}"
        )

    return results


# ---------------------------------------------------------------------------
# Game archives (PubAPI)
# ---------------------------------------------------------------------------

def fetch_player_archives(username, num_months=3):
    """
    Fetch the most recent `num_months` monthly game archives for `username`
    via the official PubAPI.

    Returns a list of dicts, one per month, each containing the month URL
    and the list of games.

    NOTE: The PubAPI game archives contain standard chess variants (chess,
    chess960, bughouse, etc.). 4-Player Chess games are NOT currently
    included in the PubAPI archives. This tool fetches all available game
    data for the top 4PC-rated players so you can study their overall play.
    """
    print(f"\n  Fetching archives for {username}...")

    # Step 1: get list of archive months
    archives_url = f"{PUBAPI_BASE}/player/{username.lower()}/games/archives"
    data, _, _ = api_get(archives_url)
    if data is None:
        print(f"    [!] No archives found for {username}")
        return []

    archive_urls = data.get("archives", [])
    if not archive_urls:
        print(f"    [!] Empty archive list for {username}")
        return []

    # Take the most recent N months
    recent = archive_urls[-num_months:]
    results = []

    for month_url in recent:
        # Extract YYYY/MM from URL for display
        parts = month_url.rstrip("/").split("/")
        year_month = f"{parts[-2]}/{parts[-1]}"
        print(f"    Fetching {year_month}...", end=" ")

        month_data, _, _ = api_get(month_url)
        if month_data is None:
            print("skipped (no data)")
            continue

        games = month_data.get("games", [])
        print(f"{len(games)} games")

        results.append({
            "month": year_month,
            "url": month_url,
            "game_count": len(games),
            "games": games,
        })

        # Brief pause between archive fetches to be polite
        time.sleep(0.5)

    return results


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

def save_output(output_dir, leaderboard, player_archives):
    """Write leaderboard and game data to JSON files in `output_dir`."""
    os.makedirs(output_dir, exist_ok=True)

    # Leaderboard summary
    lb_path = os.path.join(output_dir, "leaderboard.json")
    with open(lb_path, "w", encoding="utf-8") as f:
        json.dump(
            {
                "fetched_at": datetime.utcnow().isoformat() + "Z",
                "variant": "4-player-chess",
                "source": LEADERBOARD_URL,
                "players": leaderboard,
            },
            f,
            indent=2,
            ensure_ascii=False,
        )
    print(f"\n  Leaderboard -> {lb_path}")

    # Per-player game archives
    for username, archives in player_archives.items():
        if not archives:
            continue

        # Strip games list for the summary; full games go in a separate file
        player_dir = os.path.join(output_dir, username.lower())
        os.makedirs(player_dir, exist_ok=True)

        summary = []
        for month_data in archives:
            month_summary = {
                "month": month_data["month"],
                "url": month_data["url"],
                "game_count": month_data["game_count"],
            }
            summary.append(month_summary)

            # Full game data per month
            games_path = os.path.join(
                player_dir,
                f"games_{month_data['month'].replace('/', '-')}.json",
            )
            with open(games_path, "w", encoding="utf-8") as f:
                json.dump(month_data["games"], f, indent=2, ensure_ascii=False)

        # Player summary
        summary_path = os.path.join(player_dir, "summary.json")
        with open(summary_path, "w", encoding="utf-8") as f:
            json.dump(
                {"username": username, "archives_fetched": summary},
                f,
                indent=2,
                ensure_ascii=False,
            )
        print(f"  {username:<20s} -> {player_dir}/ ({len(archives)} months)")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description=(
            "Fetch the top 10 Chess.com 4-Player Chess players and their "
            "recent game archives via the PubAPI."
        )
    )
    parser.add_argument(
        "--months",
        type=int,
        default=3,
        help="Number of recent monthly archives to fetch per player (default: 3)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default="chess4pc_output",
        help="Output directory (default: chess4pc_output)",
    )
    args = parser.parse_args()

    # 1. Leaderboard
    leaderboard = fetch_leaderboard(count=10)

    # 2. Game archives for each player
    print(f"\n{'='*60}")
    print(f"  Fetching Game Archives (last {args.months} months per player)")
    print(f"{'='*60}")

    player_archives = {}
    for player in leaderboard:
        username = player["username"]
        archives = fetch_player_archives(username, num_months=args.months)
        player_archives[username] = archives
        # Brief pause between players
        time.sleep(1)

    # 3. Save results
    print(f"\n{'='*60}")
    print(f"  Saving Results")
    print(f"{'='*60}")

    output_dir = os.path.join(
        os.path.dirname(os.path.abspath(__file__)), args.output
    )
    save_output(output_dir, leaderboard, player_archives)

    # 4. Summary
    total_games = sum(
        m["game_count"]
        for archives in player_archives.values()
        for m in archives
    )
    print(f"\n{'='*60}")
    print(f"  Done!")
    print(f"  Players: {len(leaderboard)}")
    print(f"  Total games fetched: {total_games}")
    print(f"  Output directory: {output_dir}")
    print(f"{'='*60}")

    print(
        "\nNOTE: The Chess.com PubAPI game archives contain standard chess\n"
        "variants (chess, chess960, bughouse, etc.). 4-Player Chess games\n"
        "are not currently exposed through the PubAPI. The games fetched\n"
        "above represent the overall play of the top-rated 4PC players.\n"
    )


if __name__ == "__main__":
    main()
