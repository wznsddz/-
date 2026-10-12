<div align="center">

# 新番挂件

**常驻桌面的追番小挂件 —— 开机即见，今天更新什么一目了然。**

[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white)](https://github.com/wznsddz/-/releases/latest)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.99-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/wznsddz/-?label=release&color=ff6b9d)](https://github.com/wznsddz/-/releases/latest)

[下载](#下载) · [功能](#功能) · [本地开发](#本地开发) · [常见问题](#常见问题)

</div>

---

## 简介

一个 Windows 桌面小挂件，用最省事的方式回答一个问题：**今天有哪些动漫更新、几点更新？**

- 数据来自 [yuc.wiki（長門番堂）](https://yuc.wiki/) 当季新番表
- 无边框半透明挂件，可置顶、可拖动、可缩放
- 支持维护「在看」清单、按星期查看、CST / JST 时区切换、开机自启
- 无官方 API 依赖，本地缓存，断网也能看上次的数据

## 下载

前往 [**Releases**](https://github.com/wznsddz/-/releases/latest) 下载最新版：

| 文件 | 说明 |
| --- | --- |
| `anime-widget.exe` | 绿色便携版，双击即用，**无需安装** |

直链（始终指向最新版）：

```
https://github.com/wznsddz/-/releases/latest/download/anime-widget.exe
```

> 运行前请确保系统已安装 **WebView2 Runtime**（Windows 10/11 通常已内置；如缺失可从 [微软官网](https://developer.microsoft.com/microsoft-edge/webview2/) 获取）。

## 功能

- **按星期查看** — 顶部 `一 ~ 日` 一排按钮，默认停在今天，点哪看哪
- **今日更新状态** — 今天的列表自动区分「已更新」与「待更新」并用颜色标注
- **在看清单** — 每条右侧一键加入或移出；默认只显示「在看」的番，可一键切到「全部」
- **时区切换** — 顶栏 `CST / JST` 按钮，实时换算放送时间
- **点击番名** — 直接在浏览器打开该番的播放页
- **托盘图标** — 右键菜单支持 显示/隐藏、立即刷新、设置、退出
- **开机自启** — 设置页开关
- **本地缓存** — 抓取失败时降级显示上一次数据并在状态栏提示

## 界面说明

| 界面元素 | 作用 |
| --- | --- |
| `新番` + 年月 | 当前季度标签 |
| `CST / JST` | 点击切换时区 |
| `在看 / 全部` | 切换是否只显示在看清单 |
| `⟳` | 手动刷新数据 |
| `⚙` | 打开设置面板 |
| `—` | 隐藏窗口到托盘 |
| 顶部空白区 | 按住可拖动窗口 |

设置面板可配置：时区、默认是否只看在看、数据缓存时长（小时）、开机自启，以及管理「在看」清单。

## 技术栈

| 层 | 技术 |
| --- | --- |
| 框架 | [Tauri 2](https://tauri.app) |
| 后端 | Rust — `reqwest` 抓取、`scraper` 解析、`serde` 序列化、`chrono` 时间 |
| 前端 | 原生 HTML / CSS / JavaScript（无构建工具） |
| 插件 | `tauri-plugin-opener`、`tauri-plugin-autostart` |

## 本地开发

### 环境要求

- Node.js（含 npm）
- Rust 工具链（`rustup`，MSVC 目标）
- Microsoft C++ Build Tools（MSVC 编译器 + Windows SDK）
- WebView2 Runtime

> 校验：`cargo --version`、`node --version` 均能正常输出即可。

### 命令

```bash
npm install          # 首次安装前端依赖
npm run tauri dev    # 开发运行（弹出挂件窗口）
npm run tauri build  # 打包发布版
```

打包产物：

- 可执行文件：`src-tauri/target/release/anime-widget.exe`
- 安装包：`src-tauri/target/release/bundle/`（msi / nsis）

> 若提示 `failed to run 'cargo metadata' ... program not found`，说明当前终端是在安装 Rust 之前打开的，PATH 是旧的。**重开终端**（IDE 内需完全重启 IDE）后重试。

## 项目结构

```
.
├─ src/                      # 前端（挂件 UI）
│  ├─ index.html
│  ├─ styles.css
│  └─ main.js
├─ src-tauri/                # Rust 后端
│  ├─ src/
│  │  ├─ lib.rs              # 抓取 / 解析 / 存储 / 命令 / 托盘
│  │  └─ main.rs
│  ├─ capabilities/
│  │  └─ default.json        # 权限配置
│  ├─ icons/                 # 应用图标
│  ├─ Cargo.toml
│  └─ tauri.conf.json        # 窗口与打包配置
├─ package.json
└─ README.md
```

## 数据来源与解析

抓取地址：`https://yuc.wiki/{YYYYMM}/`（如 `https://yuc.wiki/202610/`）。

页面顶部有一张**按星期分组、精确到分钟的放送时间表**，每个番剧卡片解析出以下字段：

| 字段 | 说明 |
| --- | --- |
| `title` | 番名 |
| `weekday` | 星期（1=周一 … 7=周日） |
| `time` | 放送时间，如 `22:45`；原站用 `24:35`、`26:15` 表示**次日凌晨**，此处原样保留 |
| `first_air` | 首播日期，如 `10/5`（个别条目为 `(全24话)` 之类） |
| `platform` | 播放平台，如 `港台`、`B站` |
| `cover` | 封面图地址 |
| `link` | 播放页链接（点击番名时打开） |

解析对原站的 class 做了**前缀容错**，以兼容不同放送类型使用不同样式的情况：

- 时间：`p[class^='imgtext']`（覆盖 `imgtext4`、`imgtext5` …）
- 首播：`p[class^='imgep']`（覆盖 `imgep`、`imgep2` …）
- 标题：`td[class*='date_title']`（覆盖 `date_title_`、`date_title__`）

DOM 遍历采用**深度优先 + 文档顺序**：遇到星期表头（`td.date2`）更新当前星期，遇到番剧卡片（`div.div_date`）解析一条记录，保证每条记录归属的星期正确。

> 若某字段原站留空，前端显示 `—`；解析整体失败时降级展示缓存数据。

## 时区与季度

**时区**：原站时间基准为**北京时间（CST, UTC+8）**。选择 `CST` 原样显示；选择 `JST` 显示为北京时间 +1 小时。「已更新 / 待更新」的判定与显示使用同一时区。

**季度**：由系统日期实时计算，无需手动干预。

| 月份 | 季度号 |
| --- | --- |
| 1 ~ 3 月 | `{年}01` |
| 4 ~ 6 月 | `{年}04` |
| 7 ~ 9 月 | `{年}07` |
| 10 ~ 12 月 | `{年}10` |

例如 2027-01-01 会自动抓取 `https://yuc.wiki/202701/`。季度变化后缓存自动失效并重新抓取；若新季度页面尚未发布（404），会降级显示上一季度缓存并提示。

## 本地存储

配置与缓存位于 `%APPDATA%\com.wzn.animewidget\`：

```
config.json   # 在看清单、时区、缓存时长等偏好
cache.json    # 最近一次抓取的季度数据 + 时间戳（含缓存版本号）
```

`config.json` 结构：

```json
{
  "watching": ["番名A", "番名B"],
  "only_watching": true,
  "timezone": "CST",
  "refresh_minutes": 360
}
```

> 「在看」清单以**番名**作为唯一标识，同季度内稳定。

## 后端命令一览

前端通过 Tauri `invoke` 调用，定义于 [`src-tauri/src/lib.rs`](src-tauri/src/lib.rs)：

| 命令 | 作用 |
| --- | --- |
| `get_config` | 读取配置 |
| `save_config` | 保存配置 |
| `load_data(force)` | 获取季度数据（`force=true` 强制刷新） |
| `open_url(url)` | 用默认浏览器打开链接 |
| `set_autostart(enable)` | 设置开机自启 |
| `get_autostart` | 查询开机自启状态 |

## 常见问题

**启动时提示 `failed to run 'cargo metadata' ... program not found`？**
终端环境变量是旧的。重开终端（IDE 内需重启 IDE），或临时执行 `set PATH=%USERPROFILE%\.cargo\bin;%PATH%`。

**某些番的时间显示为 `—`？**
原站未标注放送时间（多为网络配信或档期未定）。悬停该时间可查看提示。

**`24:35`、`26:15` 是什么意思？**
原站用这种写法表示**次日凌晨**（`26:15` = 凌晨 2:15），本项目原样保留。

**改动了解析逻辑，但界面还是旧数据？**
程序优先命中缓存。已在缓存里加入版本号，解析逻辑变更时递增 `CACHE_VERSION` 即会让旧缓存自动失效；也可点顶栏 `⟳` 强制刷新。

**跨季度后数据没更新？**
点顶栏 `⟳` 刷新一次即可，季度号会按当前日期重新计算。

## 已知限制

- 依赖第三方站点结构：yuc.wiki 为个人维护站点，若页面结构大改，解析会失效（需更新选择器）。
- 无官方 API：仅抓取 HTML，请合理控制刷新频率。
- 窗口位置暂不记忆，每次启动为默认位置。
- 首次启动需联网抓取，无网络时仅能展示缓存。

---

<div align="center">

数据来源 [yuc.wiki](https://yuc.wiki/)，版权归原作者所有。

</div>
