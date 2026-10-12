const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const WEEK_LABEL = ["一", "二", "三", "四", "五", "六", "日"];

const state = {
  config: null,
  data: null,
  weekday: todayWeekday(),
  loading: false,
};

function todayWeekday() {
  // JS: 0=周日..6=周六 -> 1=周一..7=周日
  return ((new Date().getDay() + 6) % 7) + 1;
}

function pad(n) {
  return String(n).padStart(2, "0");
}

function parseHM(t) {
  if (!t || !t.includes(":")) return null;
  const [h, m] = t.split(":").map(Number);
  if (isNaN(h) || isNaN(m)) return null;
  return h * 60 + m;
}

// 原始放送时间为北京时间(CST)，按所选时区换算显示时间
function displayTime(t, tz) {
  const mins = parseHM(t);
  if (mins === null) return "—";
  const v = tz === "JST" ? mins + 60 : mins;
  return `${pad(Math.floor(v / 60))}:${pad(v % 60)}`;
}

function timeMinutes(t, tz) {
  const mins = parseHM(t);
  if (mins === null) return null;
  return tz === "JST" ? mins + 60 : mins;
}

function nowMinutes(tz) {
  const d = new Date();
  const mins = d.getHours() * 60 + d.getMinutes();
  return tz === "JST" ? mins + 60 : mins;
}

function seasonLabel(season) {
  if (!season || season.length < 6) return "";
  return `${season.slice(0, 4)}年${parseInt(season.slice(4), 10)}月`;
}

function byWeekday(wd) {
  const items = (state.data?.items || []).filter((i) => i.weekday === wd);
  // 无放送时间的条目排到最后
  const key = (i) => parseHM(i.time) ?? Number.MAX_SAFE_INTEGER;
  items.sort((a, b) => key(a) - key(b));
  return items;
}

// ---------- 渲染 ----------

function renderHeader() {
  const cfg = state.config;
  document.getElementById("season").textContent = state.data
    ? seasonLabel(state.data.season)
    : "";

  const tzBtn = document.getElementById("tz-btn");
  tzBtn.textContent = cfg.timezone;
  tzBtn.title = cfg.timezone === "JST" ? "当前：日本时间，点击切到北京时间" : "当前：北京时间，点击切到日本时间";

  const filterBtn = document.getElementById("filter-btn");
  filterBtn.textContent = cfg.only_watching ? "在看" : "全部";
  filterBtn.classList.toggle("on", cfg.only_watching);
}

function renderWeekdays() {
  const nav = document.getElementById("weekdays");
  nav.innerHTML = "";
  const today = todayWeekday();
  for (let wd = 1; wd <= 7; wd++) {
    const btn = document.createElement("button");
    btn.textContent = WEEK_LABEL[wd - 1];
    if (wd === state.weekday) btn.classList.add("active");
    if (wd === today) btn.classList.add("today");
    btn.addEventListener("click", () => {
      state.weekday = wd;
      render();
    });
    nav.appendChild(btn);
  }
}

function renderList() {
  const cfg = state.config;
  const list = document.getElementById("list");
  list.innerHTML = "";

  if (state.loading && !state.data) {
    list.innerHTML = '<div class="loading">加载中…</div>';
    return;
  }

  const isToday = state.weekday === todayWeekday();
  const tz = cfg.timezone;
  const nm = nowMinutes(tz);

  let items = byWeekday(state.weekday);
  const total = items.length;
  if (cfg.only_watching) {
    items = items.filter((i) => cfg.watching.includes(i.title));
  }

  if (items.length === 0) {
    const div = document.createElement("div");
    div.className = "empty";
    if (total === 0) {
      div.textContent = state.data?.error
        ? "暂无数据，刷新失败。"
        : "这一天没有新番。";
    } else if (cfg.only_watching) {
      div.innerHTML = `这一天共有 ${total} 部，但都不在「在看」清单中。`;
      const b = document.createElement("button");
      b.textContent = "显示全部";
      b.addEventListener("click", async () => {
        state.config.only_watching = false;
        await invoke("save_config", { config: state.config });
        render();
      });
      div.appendChild(document.createElement("br"));
      div.appendChild(b);
    }
    list.appendChild(div);
    return;
  }

  for (const it of items) {
    const row = document.createElement("div");
    row.className = "row";

    if (isToday) {
      const t = timeMinutes(it.time, tz);
      if (t !== null) row.classList.add(nm >= t ? "updated" : "pending");
    }

    const time = document.createElement("span");
    time.className = "t";
    time.textContent = displayTime(it.time, tz);
    if (parseHM(it.time) === null) {
      time.title = "原站未标注放送时间（多为网络配信或时间未定）";
    }

    const name = document.createElement("span");
    name.className = "n";
    name.textContent = it.title;
    name.title = `${it.title}\n首播：${it.first_air || "?"}${
      it.platform ? "\n平台：" + it.platform : ""
    }\n点击打开播放页`;
    name.addEventListener("click", () => {
      if (it.link) invoke("open_url", { url: it.link });
    });

    row.appendChild(time);
    row.appendChild(name);

    if (it.platform) {
      const plat = document.createElement("span");
      plat.className = "plat";
      plat.textContent = it.platform;
      row.appendChild(plat);
    }

    const star = document.createElement("button");
    star.className = "star" + (cfg.watching.includes(it.title) ? " on" : "");
    star.textContent = cfg.watching.includes(it.title) ? "★" : "☆";
    star.title = "加入 / 移出在看";
    star.addEventListener("click", () => toggleStar(it.title));
    row.appendChild(star);

    list.appendChild(row);
  }
}

function renderStatus() {
  const el = document.getElementById("status");
  el.innerHTML = "";
  const d = state.data;
  if (!d) {
    el.textContent = "";
    return;
  }

  const left = document.createElement("span");
  if (d.fetched_at) {
    const t = new Date(d.fetched_at * 1000);
    left.textContent = `更新于 ${pad(t.getHours())}:${pad(t.getMinutes())}`;
  }
  if (d.stale) {
    const w = document.createElement("span");
    w.className = "warn";
    w.textContent = "⚠ 使用缓存";
    w.title = d.error || "";
    left.appendChild(document.createTextNode("  "));
    left.appendChild(w);
  }
  el.appendChild(left);

  const total = (state.data?.items || []).length;
  const right = document.createElement("span");
  right.textContent = `共 ${total} 部`;
  el.appendChild(right);
}

function render() {
  renderHeader();
  renderWeekdays();
  renderList();
  renderStatus();
  renderSettings();
}

// ---------- 设置面板 ----------

function renderSettings() {
  const cfg = state.config;
  const panel = document.getElementById("settings");
  if (panel.classList.contains("hidden")) return;
  document.getElementById("set-tz").value = cfg.timezone;
  document.getElementById("set-only").checked = cfg.only_watching;
  document.getElementById("set-refresh").value = Math.max(
    1,
    Math.round(cfg.refresh_minutes / 60)
  );
  renderWatching();
}

function renderWatching() {
  const box = document.getElementById("watching-list");
  box.innerHTML = "";
  const list = state.config.watching;
  if (list.length === 0) {
    box.innerHTML = '<div class="hint">还没有番剧，点击列表里的 ☆ 加入。</div>';
    return;
  }
  for (const title of list) {
    const item = document.createElement("div");
    item.className = "watching-item";
    const s = document.createElement("span");
    s.textContent = title;
    s.title = title;
    const b = document.createElement("button");
    b.textContent = "✕";
    b.title = "移除";
    b.addEventListener("click", () => toggleStar(title));
    item.appendChild(s);
    item.appendChild(b);
    box.appendChild(item);
  }
}

async function openSettings(open) {
  document.getElementById("settings").classList.toggle("hidden", !open);
  if (open) {
    const enabled = await invoke("get_autostart").catch(() => false);
    document.getElementById("set-autostart").checked = enabled;
    renderSettings();
  }
}

// ---------- 交互 ----------

async function toggleStar(title) {
  const w = state.config.watching;
  const i = w.indexOf(title);
  if (i >= 0) w.splice(i, 1);
  else w.push(title);
  await invoke("save_config", { config: state.config });
  render();
}

async function loadData(force = false) {
  state.loading = true;
  if (!state.data) renderList();
  try {
    state.data = await invoke("load_data", { force });
  } catch (e) {
    state.data = { season: "", fetched_at: 0, stale: true, error: String(e), items: [] };
  } finally {
    state.loading = false;
    render();
  }
}

function bind() {
  document.getElementById("tz-btn").addEventListener("click", async () => {
    state.config.timezone = state.config.timezone === "JST" ? "CST" : "JST";
    await invoke("save_config", { config: state.config });
    render();
  });

  document.getElementById("filter-btn").addEventListener("click", async () => {
    state.config.only_watching = !state.config.only_watching;
    await invoke("save_config", { config: state.config });
    render();
  });

  document.getElementById("refresh-btn").addEventListener("click", () => loadData(true));
  document.getElementById("settings-btn").addEventListener("click", () => openSettings(true));
  document.getElementById("settings-close").addEventListener("click", () => openSettings(false));

  document.getElementById("hide-btn").addEventListener("click", () => {
    window.__TAURI__.window.getCurrentWindow().hide();
  });

  document.getElementById("set-tz").addEventListener("change", async (e) => {
    state.config.timezone = e.target.value;
    await invoke("save_config", { config: state.config });
    render();
  });

  document.getElementById("set-only").addEventListener("change", async (e) => {
    state.config.only_watching = e.target.checked;
    await invoke("save_config", { config: state.config });
    render();
  });

  document.getElementById("set-refresh").addEventListener("change", async (e) => {
    const hours = Math.max(1, parseInt(e.target.value, 10) || 6);
    state.config.refresh_minutes = hours * 60;
    await invoke("save_config", { config: state.config });
    renderSettings();
  });

  document.getElementById("set-autostart").addEventListener("change", async (e) => {
    try {
      await invoke("set_autostart", { enable: e.target.checked });
    } catch (err) {
      alert("设置开机自启失败：" + err);
    }
  });

  listen("menu://refresh", () => loadData(true));
  listen("menu://settings", () => openSettings(true));
}

async function init() {
  bind();
  state.config = await invoke("get_config");
  render();
  await loadData(false);
}

init();
