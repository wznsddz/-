use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use scraper::{ElementRef, Html, Selector};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

// ---------- 数据模型 ----------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Anime {
    pub title: String,
    pub weekday: u8,     // 1=周一 .. 7=周日
    pub time: String,    // 原始放送时间，如 "20:30"
    pub first_air: String, // 首播日期，如 "10/5"
    pub platform: String,
    pub cover: String,
    pub link: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub watching: Vec<String>,
    pub only_watching: bool,
    pub timezone: String,       // "CST"(北京时间，原站基准) | "JST"
    pub refresh_minutes: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watching: Vec::new(),
            only_watching: true,
            timezone: "CST".to_string(),
            refresh_minutes: 360,
        }
    }
}

// 解析逻辑变更时递增，使旧缓存自动失效
const CACHE_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Cache {
    #[serde(default)]
    version: u32,
    season: String,
    fetched_at: i64,
    items: Vec<Anime>,
}

#[derive(Serialize, Clone)]
pub struct DataResponse {
    season: String,
    fetched_at: i64,
    stale: bool,
    error: Option<String>,
    items: Vec<Anime>,
}

// ---------- 本地存储 ----------

fn data_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn config_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("config.json")
}

fn cache_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("cache.json")
}

fn read_config(app: &AppHandle) -> Config {
    fs::read_to_string(config_file(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_config(app: &AppHandle, cfg: &Config) -> Result<(), String> {
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_file(app), s).map_err(|e| e.to_string())
}

fn read_cache(app: &AppHandle) -> Option<Cache> {
    fs::read_to_string(cache_file(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn write_cache(app: &AppHandle, c: &Cache) {
    if let Ok(s) = serde_json::to_string(c) {
        let _ = fs::write(cache_file(app), s);
    }
}

// ---------- 季度推算 ----------

fn current_season() -> String {
    use chrono::{Datelike, Local};
    let now = Local::now();
    let sm = match now.month() {
        1..=3 => 1,
        4..=6 => 4,
        7..=9 => 7,
        _ => 10,
    };
    format!("{}{:02}", now.year(), sm)
}

// ---------- 抓取与解析 ----------

async fn fetch_season(season: &str) -> Result<Vec<Anime>, String> {
    let url = format!("https://yuc.wiki/{}/", season);
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AnimeWidget/0.1")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let html = resp.text().await.map_err(|e| e.to_string())?;
    parse_timetable(&html)
}

fn parse_weekday(s: &str) -> Option<u8> {
    const MAP: [(&str, u8); 7] = [
        ("周一", 1),
        ("周二", 2),
        ("周三", 3),
        ("周四", 4),
        ("周五", 5),
        ("周六", 6),
        ("周日", 7),
    ];
    MAP.iter().find(|(k, _)| s.contains(k)).map(|(_, v)| *v)
}

fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_timetable(html: &str) -> Result<Vec<Anime>, String> {
    let doc = Html::parse_document(html);
    let title_sel = Selector::parse("td[class*='date_title']").unwrap();
    let area_sel = Selector::parse("p.area").unwrap();
    let link_sel = Selector::parse("a[href]").unwrap();
    let time_sel = Selector::parse("p[class^='imgtext']").unwrap();
    let air_sel = Selector::parse("p[class^='imgep']").unwrap();
    let img_sel = Selector::parse("img").unwrap();

    let mut items = Vec::new();
    traverse(
        doc.root_element(),
        &mut 0u8,
        &mut items,
        &title_sel,
        &area_sel,
        &link_sel,
        &time_sel,
        &air_sel,
        &img_sel,
    );

    if items.is_empty() {
        return Err("解析失败：未找到番剧数据（页面结构可能已变化）".to_string());
    }
    Ok(items)
}

// 按文档顺序深度优先遍历：遇到日期表头更新当前星期，遇到番剧卡片解析一条记录
fn traverse(
    el: ElementRef,
    cur_wd: &mut u8,
    items: &mut Vec<Anime>,
    title_sel: &Selector,
    area_sel: &Selector,
    link_sel: &Selector,
    time_sel: &Selector,
    air_sel: &Selector,
    img_sel: &Selector,
) {
    let v = el.value();
    let tag = v.name();
    let class = v.attr("class").unwrap_or("");

    if tag == "td" && class.contains("date2") {
        if let Some(wd) = parse_weekday(&el.text().collect::<String>()) {
            *cur_wd = wd;
        }
    } else if tag == "div" && class.contains("div_date") && *cur_wd != 0 {
        let time = el
            .select(time_sel)
            .next()
            .map(|e| clean(&e.text().collect::<String>()))
            .unwrap_or_default();
        let time = time.trim_end_matches('~').trim().to_string();
        let first_air = el
            .select(air_sel)
            .next()
            .map(|e| clean(&e.text().collect::<String>()))
            .unwrap_or_default();
        let first_air = first_air.trim_end_matches('~').trim().to_string();
        let cover = el
            .select(img_sel)
            .next()
            .and_then(|e| e.value().attr("data-src").or_else(|| e.value().attr("src")))
            .unwrap_or("")
            .to_string();

        // 卡片标题/平台/链接在其父级（float:left 容器）内
        let mut title = String::new();
        let mut platform = String::new();
        let mut link = String::new();
        if let Some(parent) = el.parent().and_then(ElementRef::wrap) {
            title = parent
                .select(title_sel)
                .next()
                .map(|e| clean(&e.text().collect::<String>()))
                .unwrap_or_default();
            platform = parent
                .select(area_sel)
                .next()
                .map(|e| clean(&e.text().collect::<String>()))
                .unwrap_or_default();
            link = parent
                .select(link_sel)
                .next()
                .and_then(|e| e.value().attr("href"))
                .unwrap_or("")
                .to_string();
        }

        if !title.is_empty() {
            items.push(Anime {
                title,
                weekday: *cur_wd,
                time,
                first_air,
                platform,
                cover,
                link,
            });
        }
    }

    for child in el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            traverse(
                child_el, cur_wd, items, title_sel, area_sel, link_sel, time_sel, air_sel, img_sel,
            );
        }
    }
}

// ---------- 命令 ----------

#[tauri::command]
fn get_config(app: AppHandle) -> Config {
    read_config(&app)
}

#[tauri::command]
fn save_config(app: AppHandle, config: Config) -> Result<(), String> {
    write_config(&app, &config)
}

#[tauri::command]
async fn load_data(app: AppHandle, force: bool) -> DataResponse {
    let season = current_season();
    let cfg = read_config(&app);
    let now = chrono::Utc::now().timestamp();
    let cache = read_cache(&app);

    // 命中缓存且未过期则直接返回
    if !force {
        if let Some(c) = &cache {
            let ttl = cfg.refresh_minutes as i64 * 60;
            if c.season == season && c.version == CACHE_VERSION && now - c.fetched_at < ttl {
                return DataResponse {
                    season: c.season.clone(),
                    fetched_at: c.fetched_at,
                    stale: false,
                    error: None,
                    items: c.items.clone(),
                };
            }
        }
    }

    match fetch_season(&season).await {
        Ok(items) => {
            write_cache(
                &app,
                &Cache {
                    version: CACHE_VERSION,
                    season: season.clone(),
                    fetched_at: now,
                    items: items.clone(),
                },
            );
            DataResponse {
                season,
                fetched_at: now,
                stale: false,
                error: None,
                items,
            }
        }
        Err(e) => match cache {
            Some(c) => DataResponse {
                season: c.season,
                fetched_at: c.fetched_at,
                stale: true,
                error: Some(e),
                items: c.items,
            },
            None => DataResponse {
                season,
                fetched_at: 0,
                stale: true,
                error: Some(e),
                items: Vec::new(),
            },
        },
    }
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: AppHandle, enable: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enable {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

// ---------- 入口 ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            load_data,
            open_url,
            set_autostart,
            get_autostart
        ])
        .setup(|app| {
            let toggle = MenuItem::with_id(app, "toggle", "显示 / 隐藏", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &refresh, &settings, &quit])?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle" => {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                    "refresh" => {
                        let _ = app.emit("menu://refresh", ());
                    }
                    "settings" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                        let _ = app.emit("menu://settings", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


