# 嗅影库品牌素材

| 文件 | 用途 |
|------|------|
| `app_icon.svg` | 应用图标源文件（1024×1024，圆角方形） |
| `logo.svg` | 横版 Logo（图标 + 「嗅影库」字标；中文已转为路径，不依赖系统字体） |

## 导出平台图标

macOS `AppIcon.appiconset` 需要 PNG。在已安装 [librsvg](https://formulae.brew.sh/formula/librsvg) 时：

```bash
cd app/assets/branding
for size in 16 32 64 128 256 512 1024; do
  rsvg-convert -w "$size" -h "$size" app_icon.svg -o "/tmp/app_icon_${size}.png"
done
```

然后将输出复制到 `macos/Runner/Assets.xcassets/AppIcon.appiconset/` 对应文件名。

也可使用 Figma / Inkscape 打开 SVG 后导出各尺寸。
