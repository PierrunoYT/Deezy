# Example Themes

This directory contains example theme files that demonstrate the custom theme format for Deezy.

## Available Themes

### Midnight Blue (`midnight_blue.json`)
A deep blue theme inspired by the midnight sky. Features cool blue tones with excellent contrast for late-night music downloading sessions.

**Color Palette:**
- Deep blue backgrounds (#0a0e1a to #243356)
- Bright blue accent (#4a9eff)
- Cool white text (#e8f0ff)

### Forest Green (`forest_green.json`)
A calming green theme inspired by nature. Perfect for users who prefer earthy, natural tones.

**Color Palette:**
- Dark green backgrounds (#0a1410 to #244333)
- Vibrant green accent (#4ade80)
- Soft white text (#e8fff2)

### Sunset Orange (`sunset_orange.json`)
A warm orange theme inspired by beautiful sunsets. Brings warmth and energy to your music downloading experience.

**Color Palette:**
- Dark orange/brown backgrounds (#1a0e0a to #562a22)
- Bright orange accent (#ff7b3d)
- Warm white text (#fff5f0)

### Purple Haze (`purple_haze.json`)
A vibrant purple theme with dreamy, mystical vibes. Perfect for those who love rich, royal colors.

**Color Palette:**
- Deep purple backgrounds (#0f0a1a to #3b2456)
- Vibrant purple accent (#a855f7)
- Soft lavender text (#f5f0ff)

### Ocean Teal (`ocean_teal.json`)
A refreshing teal theme inspired by tropical ocean waters. Brings a cool, calming atmosphere.

**Color Palette:**
- Dark teal backgrounds (#0a1a1a to #245656)
- Bright teal accent (#14b8a6)
- Cool white text (#f0ffff)

### Crimson Red (`crimson_red.json`)
A bold red theme with intense, passionate energy. For users who want a striking, powerful look.

**Color Palette:**
- Dark red backgrounds (#1a0a0a to #562424)
- Bright red accent (#ef4444)
- Warm white text (#fff0f0)

### Golden Amber (`golden_amber.json`)
A luxurious golden theme with warm, rich amber tones. Exudes elegance and sophistication.

**Color Palette:**
- Dark amber backgrounds (#1a140a to #564324)
- Golden accent (#f59e0b)
- Warm cream text (#fffaf0)

### Rose Pink (`rose_pink.json`)
A soft pink theme with elegant, romantic aesthetics. Gentle on the eyes with a touch of charm.

**Color Palette:**
- Dark rose backgrounds (#1a0a14 to #562443)
- Bright pink accent (#ec4899)
- Soft pink text (#fff0f8)

### Slate Gray (`slate_gray.json`)
A professional gray theme with modern, minimalist design. Clean and focused for productivity.

**Color Palette:**
- Dark slate backgrounds (#0f1419 to #384356)
- Cool gray accent (#64748b)
- Crisp white text (#f1f5f9)

### Cherry Blossom (`cherry_blossom.json`)
A delicate theme inspired by Japanese cherry blossoms in spring. Soft and serene.

**Color Palette:**
- Dark rose backgrounds (#1a0f14 to #563843)
- Soft coral accent (#fb7185)
- Delicate pink text (#fff5f7)

### Cyber Neon (`cyber_neon.json`)
A futuristic cyberpunk theme with electric neon accents. Bold and eye-catching for a sci-fi aesthetic.

**Color Palette:**
- Deep dark backgrounds (#0a0a1a to #242456)
- Electric cyan accent (#00ffff)
- Bright cyan text (#f0ffff)

## How to Use

### Method 1: Import via Deezy UI
1. Open Deezy and go to Settings
2. Scroll to the Custom Themes section
3. Click "Import Theme"
4. Select one of these JSON files
5. Click "Apply" to activate the theme

### Method 2: Copy to Themes Directory
1. Copy the JSON file to your Deezy themes directory:
   - **Windows**: `%APPDATA%/Deezy/themes/`
   - **macOS**: `~/Library/Application Support/Deezy/themes/`
   - **Linux**: `~/.local/share/Deezy/themes/`
2. Restart Deezy or refresh the themes list
3. The theme will appear in the Custom Themes section

### Method 3: Use "Add Examples" Button
1. Open Deezy and go to Settings
2. Scroll to the Custom Themes section
3. Click "Add Examples"
4. All example themes will be automatically installed

## Customizing These Themes

Feel free to use these themes as starting points for your own custom themes:

1. Copy one of these JSON files
2. Rename it (e.g., `my_custom_theme.json`)
3. Edit the color values to your liking
4. Update the name, author, and description fields
5. Import it into Deezy

## Theme Structure

Each theme file must include:
- `name`: Display name of the theme
- `version`: Version number (e.g., "1.0.0")
- `colors`: Object with 15 required color properties

Optional fields:
- `author`: Creator's name
- `description`: Brief description of the theme

## Color Properties

All themes must define these 15 color properties:
- `bg-darkest`, `bg-dark`, `bg-surface`, `bg-elevated`, `bg-hover`
- `accent`, `accent-hover`, `accent-dim`
- `text-primary`, `text-secondary`, `text-tertiary`
- `success`, `error`, `warning`
- `border`

## Tips for Creating Themes

1. **Maintain Contrast**: Ensure text is readable against backgrounds
2. **Consistent Palette**: Use colors from the same family
3. **Test Hover States**: Make sure interactive elements are clear
4. **Use Color Tools**: Try [Coolors](https://coolors.co/) or [Adobe Color](https://color.adobe.com/)
5. **Start Simple**: Begin with one of these examples and modify gradually

## Sharing Your Themes

Created an awesome theme? Share it with the community!

1. Upload your theme JSON file to GitHub
2. Share it on social media or forums
3. Submit a pull request to add it to this collection

## More Information

For detailed documentation on creating custom themes, see:
- [CUSTOM_THEMES_GUIDE.md](../CUSTOM_THEMES_GUIDE.md) - Complete guide
- [CUSTOM_THEMES_README.md](../CUSTOM_THEMES_README.md) - Quick reference

---

**Happy theming!** 🎨
