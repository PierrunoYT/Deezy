import os
import sys

from dotenv import load_dotenv

load_dotenv()

from pydeezer import Deezer
from pydeezer.constants import track_formats
from pydeezer.exceptions import LoginError, APIRequestError

OUTPUT_FOLDER = "deezer_downloads"
QUALITY = track_formats.MP3_320


def get_deezer_client():
    arl = os.environ.get("DEEZER_ARL")
    if not arl:
        print("Error: ARL token required. Set DEEZER_ARL in .env or as an env var.")
        print("Get your ARL: Log into deezer.com > DevTools (F12) > Application > Cookies > copy 'arl' value.")
        sys.exit(1)

    try:
        return Deezer(arl=arl)
    except LoginError as e:
        print(f"Login failed: {e}")
        sys.exit(1)


def download_by_id(deezer, track_id):
    try:
        track = deezer.get_track(str(track_id))
    except APIRequestError as e:
        print(f"Could not find track with ID {track_id}: {e}")
        return

    track_data = track["info"]
    if "DATA" in track_data:
        track_data = track_data["DATA"]

    title = track_data.get("SNG_TITLE", "Unknown")
    artist = track_data.get("ART_NAME", "Unknown")
    print(f"\nDownloading: {artist} - {title}")

    try:
        track["download"](OUTPUT_FOLDER, quality=QUALITY, fallback=True)
        print(f"Saved to: {OUTPUT_FOLDER}/")
    except Exception as e:
        print(f"Download failed: {e}")


def search_and_download(deezer, query):
    results = deezer.search_tracks(query, limit=10)

    if not results:
        print("No results found.")
        return

    print(f"\nResults for \"{query}\":\n")
    for i, t in enumerate(results, 1):
        duration = int(t.get("duration", 0))
        mins, secs = divmod(duration, 60)
        print(f"  {i:>2}. {t['artist']['name']} - {t['title']}  ({mins}:{secs:02d})")

    print(f"\n   0. Cancel")

    while True:
        try:
            choice = input("\nPick a track (number): ").strip()
            if not choice:
                continue
            num = int(choice)
            if num == 0:
                print("Cancelled.")
                return
            if 1 <= num <= len(results):
                break
            print(f"Enter a number between 0 and {len(results)}.")
        except ValueError:
            print("Enter a valid number.")

    selected = results[num - 1]
    download_by_id(deezer, selected["id"])


def main():
    deezer = get_deezer_client()

    if len(sys.argv) > 1:
        arg = " ".join(sys.argv[1:])
        if arg.isdigit():
            download_by_id(deezer, int(arg))
        else:
            search_and_download(deezer, arg)
        return

    print("Deezy - Deezer Downloader\n")
    while True:
        try:
            query = input("Search (or 'q' to quit): ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\nBye!")
            break

        if not query:
            continue
        if query.lower() in ("q", "quit", "exit"):
            print("Bye!")
            break

        if query.isdigit():
            download_by_id(deezer, int(query))
        else:
            search_and_download(deezer, query)


if __name__ == "__main__":
    main()
