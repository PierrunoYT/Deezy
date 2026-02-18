# Deezy

A command-line Deezer downloader. Search for tracks, pick one, and download it as MP3 320kbps.

Includes a patched version of [pydeezer](https://github.com/Chr1st-662/pydeezer) that uses Deezer's current `media.deezer.com` API instead of the decommissioned CDN endpoints.

## Setup

**1. Install dependencies**

```bash
pip install -r requirements.txt
```

**2. Get your Deezer ARL token**

- Log into [deezer.com](https://www.deezer.com)
- Open DevTools (`F12`) > **Application** > **Cookies** > `https://www.deezer.com`
- Copy the value of the `arl` cookie

**3. Create a `.env` file** in the project root:

```
DEEZER_ARL=your_arl_token_here
```

## Usage

**Interactive mode** — search and download in a loop:

```bash
python main.py
```

```
Deezy - Deezer Downloader

Search (or 'q' to quit): Daft Punk Around The World

Results for "Daft Punk Around The World":

   1. Daft Punk - Around the World  (7:09)
   2. Daft Punk - Around the World / Harder, Better, Faster, Stronger  (5:42)
   3. Daft Punk - Around the World (Radio Edit)  (4:01)
   ...

Pick a track (number): 1
Downloading: Daft Punk - Around the World
Saved to: deezer_downloads/
```

**Search from command line:**

```bash
python main.py "Daft Punk Around The World"
```

**Download by track ID:**

```bash
python main.py 3135556
```

Downloaded tracks are saved to the `deezer_downloads/` folder as MP3 320kbps with embedded metadata and album art.
