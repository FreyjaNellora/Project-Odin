// Response emitter for Odin protocol — Stage 4
//
// Pure formatting functions: data -> String. No I/O.
// All engine responses go through these formatters.

/// Engine identification.
pub const ENGINE_NAME: &str = "Odin";
pub const ENGINE_VERSION: &str = "v0.4.1-fix";

/// Format the `id` response lines + `odinok`.
pub fn format_id() -> Vec<String> {
    vec![
        format!("id name {} {}", ENGINE_NAME, ENGINE_VERSION),
        "id author Project Odin".to_string(),
        "odinok".to_string(),
    ]
}

/// Format `readyok` response.
pub fn format_readyok() -> &'static str {
    "readyok"
}

/// Format `bestmove` response with optional ponder move.
pub fn format_bestmove(mv: &str, ponder: Option<&str>) -> String {
    match ponder {
        Some(p) => format!("bestmove {mv} ponder {p}"),
        None => format!("bestmove {mv}"),
    }
}

/// Format an error message as `info string Error: <msg>`.
pub fn format_error(msg: &str) -> String {
    format!("info string Error: {msg}")
}

/// Search info data for formatting.
/// All fields are optional — only present fields appear in the output.
/// Reserved for Stage 8+ hybrid output; currently formatted directly by BrsSearcher.
#[allow(dead_code)]
#[derive(Default)]
pub struct SearchInfo {
    pub depth: Option<u32>,
    pub seldepth: Option<u32>,
    pub score_cp: Option<i32>,
    /// Per-player values [Red, Blue, Yellow, Green].
    pub values: Option<[i32; 4]>,
    pub nodes: Option<u64>,
    pub nps: Option<u64>,
    pub time_ms: Option<u64>,
    pub pv: Vec<String>,
    /// Search phase: "brs" or "mcts".
    pub phase: Option<String>,
    pub brs_surviving: Option<u32>,
    pub mcts_sims: Option<u64>,
}

/// Format `info` string from search data.
/// Only fields that are `Some` appear in the output.
/// Reserved for Stage 8+ hybrid output; currently BrsSearcher formats its own info strings.
#[allow(dead_code)]
pub fn format_info(info: &SearchInfo) -> String {
    let mut parts = vec!["info".to_string()];

    if let Some(d) = info.depth {
        parts.push(format!("depth {d}"));
    }
    if let Some(sd) = info.seldepth {
        parts.push(format!("seldepth {sd}"));
    }
    if let Some(cp) = info.score_cp {
        parts.push(format!("score cp {cp}"));
    }
    if let Some(vals) = info.values {
        parts.push(format!(
            "v1 {} v2 {} v3 {} v4 {}",
            vals[0], vals[1], vals[2], vals[3]
        ));
    }
    if let Some(n) = info.nodes {
        parts.push(format!("nodes {n}"));
    }
    if let Some(nps) = info.nps {
        parts.push(format!("nps {nps}"));
    }
    if let Some(t) = info.time_ms {
        parts.push(format!("time {t}"));
    }
    if !info.pv.is_empty() {
        parts.push(format!("pv {}", info.pv.join(" ")));
    }
    if let Some(ref phase) = info.phase {
        parts.push(format!("phase {phase}"));
    }
    if let Some(bs) = info.brs_surviving {
        parts.push(format!("brs_surviving {bs}"));
    }
    if let Some(ms) = info.mcts_sims {
        parts.push(format!("mcts_sims {ms}"));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_id() {
        let lines = format_id();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("id name Odin"));
        assert!(lines[1].contains("author"));
        assert_eq!(lines[2], "odinok");
    }

    #[test]
    fn test_format_readyok() {
        assert_eq!(format_readyok(), "readyok");
    }

    #[test]
    fn test_format_bestmove_simple() {
        assert_eq!(format_bestmove("d4d5", None), "bestmove d4d5");
    }

    #[test]
    fn test_format_bestmove_with_ponder() {
        assert_eq!(
            format_bestmove("d4d5", Some("e7e5")),
            "bestmove d4d5 ponder e7e5"
        );
    }

    #[test]
    fn test_format_error() {
        assert_eq!(
            format_error("something went wrong"),
            "info string Error: something went wrong"
        );
    }

    #[test]
    fn test_format_info_full() {
        let info = SearchInfo {
            depth: Some(6),
            seldepth: Some(12),
            score_cp: Some(150),
            values: Some([150, -50, -30, -70]),
            nodes: Some(523847),
            nps: Some(262000),
            time_ms: Some(2000),
            pv: vec!["d4d5".to_string(), "e7e5".to_string()],
            phase: Some("brs".to_string()),
            brs_surviving: Some(5),
            mcts_sims: Some(4823),
        };
        let s = format_info(&info);
        assert!(s.starts_with("info "));
        assert!(s.contains("depth 6"));
        assert!(s.contains("seldepth 12"));
        assert!(s.contains("score cp 150"));
        assert!(s.contains("v1 150 v2 -50 v3 -30 v4 -70"));
        assert!(s.contains("nodes 523847"));
        assert!(s.contains("nps 262000"));
        assert!(s.contains("time 2000"));
        assert!(s.contains("pv d4d5 e7e5"));
        assert!(s.contains("phase brs"));
        assert!(s.contains("brs_surviving 5"));
        assert!(s.contains("mcts_sims 4823"));
    }

    #[test]
    fn test_format_info_minimal() {
        let info = SearchInfo {
            depth: Some(0),
            nodes: Some(1),
            ..Default::default()
        };
        let s = format_info(&info);
        assert_eq!(s, "info depth 0 nodes 1");
    }

    #[test]
    fn test_format_info_empty() {
        let info = SearchInfo::default();
        assert_eq!(format_info(&info), "info");
    }
}
