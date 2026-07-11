use crate::storage::Database;
use anyhow::{Context, Result};
use chrono;
use chrono::{Datelike, Timelike};
use regex::Regex;
use serde_json::Value;
use urlencoding::decode;
use walkdir::WalkDir;
use uuid::Uuid;

/// 执行 Rust 内置技能
pub async fn execute_builtin(db: &Database, name: &str, args: &Value) -> Result<String> {
    match name {
        "file_search" => search_files(args),
        "set_reminder" => set_reminder(db, args),
        "web_search" => web_search(args).await,
        "get_weather" => get_weather(args).await,
        "get_time" => get_time(args),
        "calculator" => calculator(args),
        "clipboard" => clipboard(args),
        "system_info" => system_info(args),
        "note_take" => note_take(db, args),
        "random" => random_gen(args),
        _ => Err(anyhow::anyhow!("Unknown builtin skill: {name}")),
    }
}

fn search_files(args: &Value) -> Result<String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let root_dir = args
        .get("dir")
        .and_then(|v| v.as_str())
        .unwrap_or(".");

    if query.is_empty() {
        return Ok("[]".to_string());
    }

    let mut results = Vec::new();
    for entry in WalkDir::new(root_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let fname = entry
            .file_name()
            .to_string_lossy()
            .to_lowercase();

        if fname.contains(&query.to_lowercase()) {
            results.push(entry.path().to_string_lossy().to_string());
        }
        if results.len() >= 10 {
            break;
        }
    }

    Ok(serde_json::to_string(&results)?)
}

fn set_reminder(db: &Database, args: &Value) -> Result<String> {
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 优先使用 relative_time，其次使用 fire_at
    let fire_at = if let Some(rel) = args.get("relative_time").and_then(|v| v.as_str()) {
        match parse_relative_time(rel) {
            Ok(iso) => iso,
            Err(e) => {
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("无法解析相对时间 '{}': {}", rel, e),
                })
                .to_string())
            }
        }
    } else {
        args.get("fire_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    if fire_at.is_empty() {
        return Ok(serde_json::json!({
            "status": "error",
            "message": "请提供提醒时间（fire_at 或 relative_time）",
        })
        .to_string());
    }

    let repeat = args
        .get("repeat")
        .and_then(|v| v.as_str())
        .unwrap_or("none");

    let id = format!("rem_{}", Uuid::new_v4());
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO reminder (id, persona_id, fire_at, content) VALUES (?1, '', ?2, ?3)",
            rusqlite::params![id, fire_at, content],
        )?;
        Ok(())
    })?;

    Ok(serde_json::json!({
        "status": "created",
        "id": id,
        "fire_at": fire_at,
        "content": content,
        "repeat": repeat,
    })
    .to_string())
}

/// 解析相对时间描述为 ISO 8601 格式
fn parse_relative_time(rel: &str) -> Result<String> {
    let rel = rel.trim().to_lowercase();
    let now = chrono::Local::now();

    // "in X minutes" / "in X hours" / "in X days"
    if let Some(caps) = Regex::new(r"in\s+(\d+)\s*(分钟|minute|min|分)")?.captures(&rel) {
        let n: i64 = caps[1].parse()?;
        let target = now + chrono::Duration::minutes(n);
        return Ok(target.format("%Y-%m-%dT%H:%M:%S").to_string());
    }
    if let Some(caps) = Regex::new(r"in\s+(\d+)\s*(小时|hour|hr|时)")?.captures(&rel) {
        let n: i64 = caps[1].parse()?;
        let target = now + chrono::Duration::hours(n);
        return Ok(target.format("%Y-%m-%dT%H:%M:%S").to_string());
    }
    if let Some(caps) = Regex::new(r"in\s+(\d+)\s*(天|day)")?.captures(&rel) {
        let n: i64 = caps[1].parse()?;
        let target = now + chrono::Duration::days(n);
        return Ok(target.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // "tomorrow 9am" / "tomorrow 9:00"
    if rel.starts_with("tomorrow") {
        let hour_min = parse_hour_min(&rel.replace("tomorrow", "").trim());
        let target = now + chrono::Duration::days(1);
        let target = target
            .date_naive()
            .and_hms_opt(hour_min.0, hour_min.1, 0)
            .unwrap_or_else(|| {
                now.date_naive().and_hms_opt(9, 0, 0).unwrap()
            });
        let target_dt = target.and_local_timezone(chrono::Local)
            .single()
            .unwrap_or_else(|| now);
        return Ok(target_dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    // "next Monday 3pm" / "next monday"
    if let Some(caps) = Regex::new(r"next\s+(monday|tuesday|wednesday|thursday|friday|saturday|sunday|周一|周二|周三|周四|周五|周六|周日|星期一|星期二|星期三|星期四|星期五|星期六|星期天)")?.captures(&rel) {
        let weekday_str = caps[1].to_string();
        let target_weekday = parse_weekday(&weekday_str);
        let mut days_ahead = target_weekday as i64 - now.weekday().num_days_from_monday() as i64;
        if days_ahead <= 0 {
            days_ahead += 7;
        }
        let rest = rel[caps.get(0).unwrap().end()..].trim().to_string();
        let hour_min = parse_hour_min(&rest);
        let target_date = now + chrono::Duration::days(days_ahead);
        let target = target_date
            .date_naive()
            .and_hms_opt(hour_min.0, hour_min.1, 0)
            .unwrap_or_else(|| {
                target_date.date_naive().and_hms_opt(9, 0, 0).unwrap()
            });
        let target_dt = target.and_local_timezone(chrono::Local)
            .single()
            .unwrap_or_else(|| now);
        return Ok(target_dt.format("%Y-%m-%dT%H:%M:%S").to_string());
    }

    Err(anyhow::anyhow!(
        "无法识别的相对时间格式。支持：'in 30 minutes'、'in 2 hours'、'tomorrow 9am'、'next Monday 3pm'"
    ))
}

fn parse_hour_min(s: &str) -> (u32, u32) {
    // 尝试解析 "9am", "9:00", "15:00", "下午3点" 等
    let s = s.trim().to_lowercase();

    // "9am" / "9pm"
    if let Some(caps) = Regex::new(r"(\d{1,2})\s*(am|pm)").ok().and_then(|re| re.captures(&s)) {
        let mut h: u32 = caps[1].parse().unwrap_or(9);
        let is_pm = &caps[2] == "pm";
        if is_pm && h < 12 {
            h += 12;
        }
        if !is_pm && h == 12 {
            h = 0;
        }
        return (h, 0);
    }

    // "下午3点" / "上午9点" / "晚上8点"
    if let Some(caps) = Regex::new(r"(上午|下午|晚上)?(\d{1,2})\s*点").ok().and_then(|re| re.captures(&s)) {
        let mut h: u32 = caps[2].parse().unwrap_or(9);
        match caps.get(1).map(|m| m.as_str()) {
            Some("下午") | Some("晚上") => if h < 12 { h += 12 },
            Some("上午") => if h == 12 { h = 0 },
            _ => {}
        }
        return (h, 0);
    }

    // "9:00" / "15:30"
    if let Some(caps) = Regex::new(r"(\d{1,2}):(\d{2})").ok().and_then(|re| re.captures(&s)) {
        let h: u32 = caps[1].parse().unwrap_or(9);
        let m: u32 = caps[2].parse().unwrap_or(0);
        return (h, m);
    }

    (9, 0) // default: 9:00 AM
}

fn parse_weekday(s: &str) -> u32 {
    match s {
        "monday" | "周一" | "星期一" => 0,
        "tuesday" | "周二" | "星期二" => 1,
        "wednesday" | "周三" | "星期三" => 2,
        "thursday" | "周四" | "星期四" => 3,
        "friday" | "周五" | "星期五" => 4,
        "saturday" | "周六" | "星期六" => 5,
        "sunday" | "周日" | "星期天" => 6,
        _ => 0,
    }
}

async fn web_search(args: &Value) -> Result<String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if query.is_empty() {
        return Ok(serde_json::json!({"error": "query is empty"}).to_string());
    }

    let top_n = args
        .get("top_n")
        .and_then(|v| v.as_u64())
        .unwrap_or(5)
        .clamp(1, 10) as usize;

    // 时间范围过滤参数：day/week/month/year
    let time_range = args.get("time_range").and_then(|v| v.as_str()).unwrap_or("");
    let time_filter = match time_range {
        "day" => "d",
        "week" => "w",
        "month" => "m",
        "year" => "y",
        _ => "",
    };

    // 构造 URL：如果有时间过滤，添加 df 参数（DuckDuckGo 的时间范围参数）
    let mut url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    if !time_filter.is_empty() {
        url.push_str(&format!("&df={}", time_filter));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;

    let html = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.0")
        .send()
        .await
        .context("send web_search request")?
        .text()
        .await
        .context("read web_search response")?;

    let results = parse_duckduckgo_results(&html, top_n);

    if results.is_empty() {
        return Ok(serde_json::json!({"message": "未找到相关结果", "results": []}).to_string());
    }

    Ok(serde_json::to_string(&serde_json::json!({
        "query": query,
        "time_range": if time_range.is_empty() { "all" } else { time_range },
        "results": results,
    }))?)
}

#[derive(Debug, serde::Serialize)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn parse_duckduckgo_results(html: &str, top_n: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // 匹配结果标题与链接
    let title_re = match Regex::new(r#"<a[^>]*class="result__a"[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#) {
        Ok(re) => re,
        Err(_) => return results,
    };

    // 匹配摘要
    let snippet_re = match Regex::new(r#"<a[^>]*class="result__snippet"[^>]*>(.*?)</a>"#) {
        Ok(re) => re,
        Err(_) => return results,
    };

    let titles: Vec<(String, String)> = title_re
        .captures_iter(html)
        .filter_map(|cap| {
            let href = cap.get(1)?.as_str();
            let title_html = cap.get(2)?.as_str();
            Some((normalize_url(href), strip_html_tags(title_html)))
        })
        .collect();

    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .map(|cap| strip_html_tags(cap.get(1).map(|m| m.as_str()).unwrap_or("")))
        .collect();

    for (i, (url, title)) in titles.into_iter().enumerate() {
        if i >= top_n {
            break;
        }
        let snippet = snippets.get(i).cloned().unwrap_or_default();
        results.push(SearchResult { title, url, snippet });
    }

    results
}

fn normalize_url(href: &str) -> String {
    // DuckDuckGo 经常把真实 URL 放在 /l/?...&uddg=URLENCODED 里
    if let Some(pos) = href.find("uddg=") {
        let encoded = &href[pos + 5..];
        return decode(encoded).unwrap_or_else(|_| encoded.into()).into_owned();
    }
    decode(href).unwrap_or_else(|_| href.into()).into_owned()
}

async fn get_weather(args: &Value) -> Result<String> {
    let city = args
        .get("city")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if city.is_empty() {
        return Ok(serde_json::json!({"error": "city is empty"}).to_string());
    }

    // 解析 date 参数：today / tomorrow / ISO 日期
    let date_param = args.get("date").and_then(|v| v.as_str()).unwrap_or("today");
    let days = args.get("days").and_then(|v| v.as_u64()).unwrap_or(1).clamp(1, 3) as usize;

    let query_date = match date_param {
        "today" => 1,
        "tomorrow" => 2,
        _ => {
            // 尝试解析 ISO 日期，计算距离今天的天数
            if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_param, "%Y-%m-%d") {
                let now = chrono::Local::now().date_naive();
                let diff = (parsed - now).num_days();
                if diff < 0 {
                    1 // 过去日期默认今天
                } else if diff > 2 {
                    3 // wttr.in 最多3天
                } else {
                    (diff + 1) as usize
                }
            } else {
                1
            }
        }
    };

    let url = if query_date == 1 {
        format!("https://wttr.in/{}?format=j1", urlencoding::encode(city))
    } else {
        format!("https://wttr.in/{}?format=j2", urlencoding::encode(city))
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;

    let text = client
        .get(&url)
        .header("User-Agent", "curl/7.68.0")
        .send()
        .await
        .context("send get_weather request")?
        .text()
        .await
        .context("read get_weather response")?;

    let data: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|_| serde_json::json!({"raw": text }));

    // 根据 query_date 选择当前天气或预报
    if query_date == 1 {
        if let Some(current) = data
            .get("current_condition")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
        {
            let temp = current["temp_C"].as_str().unwrap_or("--");
            let desc = current["weatherDesc"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| v["value"].as_str())
                .unwrap_or("未知");
            let humidity = current["humidity"].as_str().unwrap_or("--");
            let feels_like = current["FeelsLikeC"].as_str().unwrap_or("--");
            let wind_speed = current["windspeedKmph"].as_str().unwrap_or("--");

            return Ok(serde_json::json!({
                "city": city,
                "date": "今天",
                "temperature_c": temp,
                "feels_like_c": feels_like,
                "description": desc,
                "humidity": humidity,
                "wind_speed_kmph": wind_speed,
            })
            .to_string());
        }
    } else {
        // 预报天气
        if let Some(weather_arr) = data.get("weather").and_then(|v| v.as_array()) {
            let day_idx = query_date - 2; // j2 格式: weather[0]=明天, weather[1]=后天
            if let Some(day) = weather_arr.get(day_idx) {
                let date_str = day["date"].as_str().unwrap_or("");
                let max_temp = day["maxtempC"].as_str().unwrap_or("--");
                let min_temp = day["mintempC"].as_str().unwrap_or("--");
                let desc = day["hourly"]
                    .as_array()
                    .and_then(|arr| arr.get(4)) // 中午12点左右
                    .and_then(|h| h["weatherDesc"].as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v["value"].as_str())
                    .unwrap_or("未知");

                return Ok(serde_json::json!({
                    "city": city,
                    "date": date_str,
                    "query_date": date_param,
                    "max_temp_c": max_temp,
                    "min_temp_c": min_temp,
                    "description": desc,
                })
                .to_string());
            }
        }
    }

    Ok(serde_json::json!({
        "city": city,
        "message": "暂时无法获取天气信息",
    })
    .to_string())
}

fn strip_html_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(ch);
        }
    }
    // 合并连续空白
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ============================================================
// 新增技能实现
// ============================================================

/// get_time — 时间查询技能
fn get_time(args: &Value) -> Result<String> {
    let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("current");
    let now = chrono::Local::now();

    match operation {
        "add" => {
            let days: i64 = args.get("days").and_then(|v| v.as_i64()).unwrap_or(0);
            let target = now + chrono::Duration::days(days);
            let weekday = weekday_cn(target.weekday().num_days_from_monday());
            Ok(serde_json::json!({
                "operation": "add",
                "base_date": now.format("%Y-%m-%d").to_string(),
                "days": days,
                "result_date": target.format("%Y-%m-%d").to_string(),
                "weekday": weekday,
                "full": format!("{}天后是{}年{}月{}日，星期{}",
                    days, target.year(), target.month(), target.day(), weekday),
            }).to_string())
        }
        "diff" => {
            let date1_str = args.get("date1").and_then(|v| v.as_str()).unwrap_or("");
            let date2_str = args.get("date2").and_then(|v| v.as_str()).unwrap_or("");
            let d1 = chrono::NaiveDate::parse_from_str(date1_str, "%Y-%m-%d")
                .unwrap_or_else(|_| now.date_naive());
            let d2 = chrono::NaiveDate::parse_from_str(date2_str, "%Y-%m-%d")
                .unwrap_or_else(|_| now.date_naive());
            let diff = (d2 - d1).num_days();
            Ok(serde_json::json!({
                "operation": "diff",
                "date1": d1.format("%Y-%m-%d").to_string(),
                "date2": d2.format("%Y-%m-%d").to_string(),
                "days": diff.abs(),
                "direction": if diff >= 0 { "之后" } else { "之前" },
                "full": format!("{} 距离 {} 还有 {} 天", d1.format("%Y-%m-%d"), d2.format("%Y-%m-%d"), diff.abs()),
            }).to_string())
        }
        "countdown" => {
            let target_str = args.get("target").and_then(|v| v.as_str()).unwrap_or("");
            if let Ok(target) = chrono::NaiveDate::parse_from_str(target_str, "%Y-%m-%d") {
                let today = now.date_naive();
                let remaining = (target - today).num_days();
                Ok(serde_json::json!({
                    "operation": "countdown",
                    "target": target_str,
                    "remaining_days": remaining,
                    "full": if remaining > 0 {
                        format!("距离 {} 还有 {} 天", target_str, remaining)
                    } else if remaining == 0 {
                        format!("今天就是 {}！", target_str)
                    } else {
                        format!("{} 已经过去了 {} 天", target_str, -remaining)
                    },
                }).to_string())
            } else {
                Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("无法解析日期 '{}'，请使用 YYYY-MM-DD 格式", target_str),
                }).to_string())
            }
        }
        "timezone" => {
            let tz_name = args.get("timezone").and_then(|v| v.as_str()).unwrap_or("UTC");
            // 简易时区处理：仅支持常见偏移
            let tz_offset_h = match tz_name.to_lowercase().as_str() {
                "america/new_york" | "new york" | "ny" | "est" | "edt" => -4,
                "america/chicago" | "chicago" => -5,
                "america/denver" | "denver" => -6,
                "america/los_angeles" | "los angeles" | "la" | "pst" | "pdt" => -7,
                "europe/london" | "london" | "gmt" | "bst" => 1,
                "europe/paris" | "paris" | "europe/berlin" | "berlin" => 2,
                "europe/moscow" | "moscow" => 3,
                "asia/dubai" | "dubai" => 4,
                "asia/shanghai" | "shanghai" | "asia/tokyo" | "tokyo" | "asia/seoul" | "seoul" | "cst" | "jst" => 8,
                "australia/sydney" | "sydney" => 10,
                "pacific/auckland" | "auckland" => 12,
                _ => {
                    // 支持 "+8" / "-5" 格式
                    if let Ok(offset) = tz_name.parse::<i64>() {
                        offset
                    } else {
                        return Ok(serde_json::json!({
                            "status": "error",
                            "message": format!("不支持的时区 '{}'，请使用如 America/New_York、Asia/Tokyo 格式", tz_name),
                        }).to_string());
                    }
                }
            };
            let target_time = now + chrono::Duration::hours(tz_offset_h - 8); // 假设本地是 UTC+8
            let weekday = weekday_cn(target_time.weekday().num_days_from_monday());
            Ok(serde_json::json!({
                "operation": "timezone",
                "timezone": tz_name,
                "local_time": now.format("%Y-%m-%d %H:%M:%S").to_string(),
                "target_time": target_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                "target_weekday": weekday,
                "offset_hours": tz_offset_h,
                "full": format!("{} 当前时间：{}年{}月{}日 {}:{:02}，星期{}，UTC{:+}",
                    tz_name, target_time.year(), target_time.month(), target_time.day(),
                    target_time.hour(), target_time.minute(), weekday, tz_offset_h),
            }).to_string())
        }
        _ => {
            // current — 返回当前完整时间信息
            let weekday = weekday_cn(now.weekday().num_days_from_monday());
            let week_num = now.iso_week().week();
            let day_of_year = now.ordinal();
            let season = match now.month() {
                3 | 4 | 5 => "春",
                6 | 7 | 8 => "夏",
                9 | 10 | 11 => "秋",
                _ => "冬",
            };
            let time_of_day = match now.hour() {
                0..=5 => "凌晨",
                6..=8 => "早晨",
                9..=11 => "上午",
                12..=13 => "中午",
                14..=17 => "下午",
                18..=19 => "傍晚",
                20..=22 => "晚上",
                _ => "深夜",
            };

            Ok(serde_json::json!({
                "operation": "current",
                "datetime": now.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "year": now.year(),
                "month": now.month(),
                "day": now.day(),
                "hour": now.hour(),
                "minute": now.minute(),
                "second": now.second(),
                "weekday": weekday,
                "week_number": week_num,
                "day_of_year": day_of_year,
                "season": season,
                "time_of_day": time_of_day,
                "timestamp": now.timestamp(),
                "full": format!("现在是{}年{}月{}日 {}:{:02}:{:02}，星期{}，{}季，{}，第{}周",
                    now.year(), now.month(), now.day(),
                    now.hour(), now.minute(), now.second(),
                    weekday, season, time_of_day, week_num),
            }).to_string())
        }
    }
}

fn weekday_cn(day: u32) -> &'static str {
    match day {
        0 => "一", 1 => "二", 2 => "三", 3 => "四",
        4 => "五", 5 => "六", 6 => "日",
        _ => "?",
    }
}

/// calculator — 安全数学表达式求值
fn calculator(args: &Value) -> Result<String> {
    let expr = args.get("expression").and_then(|v| v.as_str()).unwrap_or("").trim();

    if expr.is_empty() {
        return Ok(serde_json::json!({"error": "表达式为空"}).to_string());
    }

    // 安全限制：只允许数学表达式字符
    if Regex::new(r"^[0-9+\-*/().%\s^eEpPiIsSnNcCoOtTaAqQrRlLgGfF!]+$")?
        .is_match(expr) == false
    {
        return Ok(serde_json::json!({
            "error": "表达式包含不允许的字符，仅支持数学运算",
            "expression": expr,
        }).to_string());
    }

    match expr.parse::<meval::Expr>() {
        Ok(parsed) => match parsed.eval() {
            Ok(result) => Ok(serde_json::json!({
                "expression": expr,
                "result": result,
                "full": format!("{} = {}", expr, result),
            }).to_string()),
            Err(e) => Ok(serde_json::json!({
                "error": format!("计算错误: {}", e),
                "expression": expr,
            }).to_string()),
        },
        Err(e) => Ok(serde_json::json!({
            "error": format!("表达式解析错误: {}", e),
            "expression": expr,
        }).to_string()),
    }
}

/// clipboard — 系统剪贴板读写
fn clipboard(args: &Value) -> Result<String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("read");

    match action {
        "write" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                return Ok(serde_json::json!({"status": "error", "message": "要写入的文本为空"}).to_string());
            }

            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.set_text(text) {
                        Ok(()) => Ok(serde_json::json!({
                            "status": "ok",
                            "action": "write",
                            "length": text.len(),
                            "message": format!("已将 {} 个字符写入剪贴板", text.len()),
                        }).to_string()),
                        Err(e) => Ok(serde_json::json!({
                            "status": "error",
                            "message": format!("写入剪贴板失败: {}", e),
                        }).to_string()),
                    }
                }
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("无法访问剪贴板: {}", e),
                }).to_string()),
            }
        }
        _ => {
            // read
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.get_text() {
                        Ok(text) => {
                            let display = if text.len() > 500 {
                                format!("{}...(共{}字符)", &text[..500], text.len())
                            } else {
                                text.clone()
                            };
                            Ok(serde_json::json!({
                                "status": "ok",
                                "action": "read",
                                "text": text,
                                "display": display,
                                "length": text.len(),
                            }).to_string())
                        }
                        Err(e) => Ok(serde_json::json!({
                            "status": "error",
                            "message": format!("读取剪贴板失败: {}", e),
                        }).to_string()),
                    }
                }
                Err(e) => Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("无法访问剪贴板: {}", e),
                }).to_string()),
            }
        }
    }
}

/// system_info — 系统状态查询
fn system_info(args: &Value) -> Result<String> {
    use sysinfo::System;

    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("all");
    let mut sys = System::new_all();
    sys.refresh_all();

    match query {
        "cpu" => {
            let cpu_count = sys.cpus().len();
            let cpu_usage: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
            let avg_usage = if cpu_usage.is_empty() { 0.0 } else {
                cpu_usage.iter().sum::<f32>() / cpu_usage.len() as f32
            };
            let brand = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default();
            Ok(serde_json::json!({
                "cpu_brand": brand,
                "cpu_cores": cpu_count,
                "cpu_usage_percent": format!("{:.1}", avg_usage),
                "per_core": cpu_usage.iter().map(|u| format!("{:.1}%", u)).collect::<Vec<_>>(),
                "full": format!("CPU: {}，{}核，当前使用率 {:.1}%", brand, cpu_count, avg_usage),
            }).to_string())
        }
        "memory" => {
            let total_mb = sys.total_memory() as f64 / 1_048_576.0;
            let used_mb = sys.used_memory() as f64 / 1_048_576.0;
            let free_mb = total_mb - used_mb;
            let pct = if total_mb > 0.0 { (used_mb / total_mb) * 100.0 } else { 0.0 };
            let total_swap_mb = sys.total_swap() as f64 / 1_048_576.0;
            let used_swap_mb = sys.used_swap() as f64 / 1_048_576.0;
            Ok(serde_json::json!({
                "total_memory_mb": format!("{:.0}", total_mb),
                "used_memory_mb": format!("{:.0}", used_mb),
                "free_memory_mb": format!("{:.0}", free_mb),
                "usage_percent": format!("{:.1}", pct),
                "total_swap_mb": format!("{:.0}", total_swap_mb),
                "used_swap_mb": format!("{:.0}", used_swap_mb),
                "full": format!("内存: {:.0}MB / {:.0}MB ({:.1}%)", used_mb, total_mb, pct),
            }).to_string())
        }
        "disk" => {
            let mut disks_info = Vec::new();
            for disk in sysinfo::Disks::new_with_refreshed_list().list() {
                let total_gb = disk.total_space() as f64 / 1_073_741_824.0;
                let avail_gb = disk.available_space() as f64 / 1_073_741_824.0;
                let used_gb = total_gb - avail_gb;
                let pct = if total_gb > 0.0 { (used_gb / total_gb) * 100.0 } else { 0.0 };
                disks_info.push(serde_json::json!({
                    "mount": disk.mount_point().to_string_lossy(),
                    "total_gb": format!("{:.1}", total_gb),
                    "used_gb": format!("{:.1}", used_gb),
                    "free_gb": format!("{:.1}", avail_gb),
                    "usage_percent": format!("{:.1}", pct),
                }));
            }
            Ok(serde_json::json!({
                "disks": disks_info,
                "full": disks_info.iter().map(|d| {
                    format!("{}: {}/{} GB", d["mount"].as_str().unwrap_or("?"),
                        d["used_gb"].as_str().unwrap_or("?"),
                        d["total_gb"].as_str().unwrap_or("?"))
                }).collect::<Vec<_>>().join("; "),
            }).to_string())
        }
        "battery" => {
            // sysinfo 在部分平台不支持电池，返回提示
            Ok(serde_json::json!({
                "message": "电池信息在当前平台不可用（桌面设备通常无电池）",
                "full": "电池信息不可用",
            }).to_string())
        }
        "os" => {
            let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
            let os_ver = System::os_version().unwrap_or_else(|| "Unknown".to_string());
            let kernel = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
            let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
            let uptime_secs = System::uptime();
            let uptime_h = uptime_secs / 3600;
            let uptime_m = (uptime_secs % 3600) / 60;
            Ok(serde_json::json!({
                "os": os_name,
                "version": os_ver,
                "kernel": kernel,
                "hostname": hostname,
                "uptime_seconds": uptime_secs,
                "uptime_display": format!("{}小时{}分钟", uptime_h, uptime_m),
                "full": format!("系统: {} {}，内核: {}，主机名: {}，已运行 {}小时{}分钟",
                    os_name, os_ver, kernel, hostname, uptime_h, uptime_m),
            }).to_string())
        }
        _ => {
            // all — 返回所有信息摘要
            let os_name = System::name().unwrap_or_default();
            let total_mb = sys.total_memory() as f64 / 1_048_576.0;
            let used_mb = sys.used_memory() as f64 / 1_048_576.0;
            let mem_pct = if total_mb > 0.0 { (used_mb / total_mb) * 100.0 } else { 0.0 };
            let cpu_usage: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
                / sys.cpus().len().max(1) as f32;
            let cpu_brand = sys.cpus().first().map(|c| c.brand()).unwrap_or("Unknown");
            let uptime_h = System::uptime() / 3600;

            Ok(serde_json::json!({
                "os": os_name,
                "cpu": format!("{} ({}核, {:.1}%)", cpu_brand, sys.cpus().len(), cpu_usage),
                "memory": format!("{:.0}MB / {:.0}MB ({:.1}%)", used_mb, total_mb, mem_pct),
                "uptime_hours": uptime_h,
                "full": format!("系统概况:\n- OS: {}\n- CPU: {} ({}核, 使用率 {:.1}%)\n- 内存: {:.0}MB / {:.0}MB ({:.1}%)\n- 运行时间: {}小时",
                    os_name, cpu_brand, sys.cpus().len(), cpu_usage, used_mb, total_mb, mem_pct, uptime_h),
            }).to_string())
        }
    }
}

/// note_take — 笔记管理
fn note_take(db: &Database, args: &Value) -> Result<String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");

    // 确保 notes 表存在
    let _ = db.with_conn(|conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS note (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                tags TEXT DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
    });

    match action {
        "save" => {
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if content.is_empty() {
                return Ok(serde_json::json!({"status": "error", "message": "笔记内容为空"}).to_string());
            }
            let tags = args.get("tags").and_then(|v| v.as_str()).unwrap_or("");
            let id = format!("note_{}", Uuid::new_v4());
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO note (id, content, tags) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id, content, tags],
                )?;
                Ok(())
            })?;
            Ok(serde_json::json!({
                "status": "ok",
                "action": "save",
                "id": id,
                "content": content,
                "tags": tags,
                "message": "笔记已保存",
            }).to_string())
        }
        "search" => {
            let keyword = args.get("keyword").and_then(|v| v.as_str()).unwrap_or("");
            if keyword.is_empty() {
                return Ok(serde_json::json!({"status": "error", "message": "搜索关键词为空"}).to_string());
            }
            let pattern = format!("%{}%", keyword);
            let results: Vec<serde_json::Value> = db.with_conn(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, content, tags, created_at FROM note WHERE content LIKE ?1 OR tags LIKE ?1 ORDER BY created_at DESC LIMIT 20"
                )?;
                let rows = stmt.query_map(rusqlite::params![pattern], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "content": row.get::<_, String>(1)?,
                        "tags": row.get::<_, String>(2)?,
                        "created_at": row.get::<_, String>(3)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            })?;
            Ok(serde_json::json!({
                "action": "search",
                "keyword": keyword,
                "count": results.len(),
                "results": results,
            }).to_string())
        }
        "delete" => {
            let note_id = args.get("note_id").and_then(|v| v.as_str()).unwrap_or("");
            if note_id.is_empty() {
                return Ok(serde_json::json!({"status": "error", "message": "笔记ID为空"}).to_string());
            }
            let deleted = db.with_conn(|conn| {
                Ok(conn.execute("DELETE FROM note WHERE id = ?1", rusqlite::params![note_id])?)
            })?;
            Ok(serde_json::json!({
                "status": if deleted > 0 { "ok" } else { "not_found" },
                "action": "delete",
                "id": note_id,
                "message": if deleted > 0 { "笔记已删除" } else { "未找到该笔记" },
            }).to_string())
        }
        _ => {
            // list — 列出所有笔记
            let results: Vec<serde_json::Value> = db.with_conn(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, content, tags, created_at FROM note ORDER BY created_at DESC LIMIT 50"
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "content": row.get::<_, String>(1)?,
                        "tags": row.get::<_, String>(2)?,
                        "created_at": row.get::<_, String>(3)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            })?;
            Ok(serde_json::json!({
                "action": "list",
                "count": results.len(),
                "results": results,
            }).to_string())
        }
    }
}

/// random — 随机生成
fn random_gen(args: &Value) -> Result<String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let gen_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("number");

    match gen_type {
        "number" => {
            let min = args.get("min").and_then(|v| v.as_i64()).unwrap_or(1);
            let max = args.get("max").and_then(|v| v.as_i64()).unwrap_or(100);
            let result = rng.gen_range(min..=max);
            Ok(serde_json::json!({
                "type": "number",
                "min": min,
                "max": max,
                "result": result,
                "full": format!("随机数（{}-{}）: {}", min, max, result),
            }).to_string())
        }
        "dice" => {
            let faces = args.get("faces").and_then(|v| v.as_i64()).unwrap_or(6).max(2);
            let count = args.get("count").and_then(|v| v.as_i64()).unwrap_or(1).clamp(1, 100) as usize;
            let results: Vec<i64> = (0..count).map(|_| rng.gen_range(1..=faces)).collect();
            let total: i64 = results.iter().sum();
            Ok(serde_json::json!({
                "type": "dice",
                "faces": faces,
                "count": count,
                "results": results,
                "total": total,
                "full": format!("{}d{}: {:?} = {}", count, faces, results, total),
            }).to_string())
        }
        "password" => {
            let length = args.get("length").and_then(|v| v.as_i64()).unwrap_or(16).clamp(4, 128) as usize;
            let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
            let password: String = (0..length).map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            }).collect();
            Ok(serde_json::json!({
                "type": "password",
                "length": length,
                "password": password,
                "full": format!("随机密码（{}位）: {}", length, password),
            }).to_string())
        }
        "pick" => {
            let items_str = args.get("items").and_then(|v| v.as_str()).unwrap_or("");
            let pick_count = args.get("pick_count").and_then(|v| v.as_i64()).unwrap_or(1).max(1) as usize;
            let items: Vec<&str> = items_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if items.is_empty() {
                return Ok(serde_json::json!({"status": "error", "message": "候选项列表为空"}).to_string());
            }
            let pick_n = pick_count.min(items.len());
            // Fisher-Yates 部分洗牌
            let mut indices: Vec<usize> = (0..items.len()).collect();
            for i in 0..pick_n {
                let j = rng.gen_range(i..indices.len());
                indices.swap(i, j);
            }
            let picked: Vec<&str> = indices[..pick_n].iter().map(|&i| items[i]).collect();
            Ok(serde_json::json!({
                "type": "pick",
                "total_items": items.len(),
                "pick_count": pick_n,
                "picked": picked,
                "full": if pick_n == 1 {
                    format!("从 {} 个选项中随机选择: {}", items.len(), picked[0])
                } else {
                    format!("从 {} 个选项中随机选择 {} 个: {:?}", items.len(), pick_n, picked)
                },
            }).to_string())
        }
        _ => Ok(serde_json::json!({
            "status": "error",
            "message": format!("不支持的随机类型 '{}'，支持: number, dice, password, pick", gen_type),
        }).to_string()),
    }
}
