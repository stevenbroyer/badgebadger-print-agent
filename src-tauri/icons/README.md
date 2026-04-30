# Icons

Tauri's bundler reads the icon set from `tauri.conf.json` →
`bundle.icon`. Drop the following files in this directory before the
first `pnpm tauri build` run:

| File | Purpose |
| --- | --- |
| `32x32.png` | tray icon, small dialogs |
| `128x128.png` | application icon |
| `128x128@2x.png` | retina application icon |
| `icon.ico` | Windows installer / EXE icon |
| `icon.icns` | macOS bundle icon |

The Tauri CLI ships an icon generator that converts a single 1024×1024
PNG into all of the above:

```sh
pnpm tauri icon path/to/source.png
```

For now we're using the BadgeBadger mascot — point the command at
`../public/logo-mascot.svg` after exporting it to PNG.
