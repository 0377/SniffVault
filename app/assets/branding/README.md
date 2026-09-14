# 嗅影库品牌素材

| 文件 | 用途 |
|------|------|
| `app_icon.svg` | 应用图标源文件（1024×1024，圆角方形） |
| `logo.svg` | 横版 Logo（图标 + 「嗅影库」字标；中文已转为路径，不依赖系统字体） |

## 导出平台图标

**务必从 `app_icon.svg` 导出。** SVG 须为有效 UTF-8（中文用 XML 实体，如 `&#x55c5;&#x5f71;&#x5e93;`）；编码错误时浏览器会生成报错页截图，不可当作图标使用。

先导出 1024×1024 主图：

```bash
cd app/assets/branding
npx --yes @resvg/resvg-js-cli app_icon.svg app_icon.png --fit-width 1024 --fit-height 1024
```

### Android

从 `app_icon.png` 生成各密度：

```bash
cd app
SRC=assets/branding/app_icon.png
DEST=android/app/src/main/res
for spec in mdpi:48 hdpi:72 xhdpi:96 xxhdpi:144 xxxhdpi:192; do
  d=${spec%%:*}; s=${spec##*:}
  sips -z $s $s "$SRC" --out "$DEST/mipmap-$d/ic_launcher.png"
done
for spec in mdpi:108 hdpi:162 xhdpi:216 xxhdpi:324 xxxhdpi:432; do
  d=${spec%%:*}; s=${spec##*:}
  sips -z $s $s "$SRC" --out "$DEST/mipmap-$d/ic_launcher_foreground.png"
done
```

### iOS / macOS

从 `app_icon.png` 用 `sips` 缩放至 `ios/Runner/Assets.xcassets/AppIcon.appiconset/` 与 `macos/Runner/Assets.xcassets/AppIcon.appiconset/` 各尺寸（见仓库内现有文件名）。

也可使用 Figma / Inkscape 打开 SVG 后导出各尺寸。
