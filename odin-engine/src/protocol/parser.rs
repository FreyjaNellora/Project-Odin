// Command parser for Odin protocol — Stage 4
//
// Pure function: &str -> Command. No I/O, no side effects.
// Handles all command variants and malformed input gracefully.

use super::types::{Command, SearchLimits};

/// Parse a raw input line into a Command.
pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    if input.is_empty() {
        return Command::Unknown(String::new());
    }

    let mut tokens = input.split_whitespace();
    let cmd = tokens
        .next()
        .expect("non-empty input has at least one token");

    match cmd {
        "odin" => Command::Odin,
        "isready" => Command::IsReady,
        "setoption" => parse_setoption(tokens),
        "position" => parse_position(tokens),
        "go" => parse_go(tokens),
        "stop" => Command::Stop,
        "quit" => Command::Quit,
        _ => Command::Unknown(input.to_string()),
    }
}

/// Parse `setoption name <name_tokens> value <value_tokens>`.
/// The name can be multi-word (everything between `name` and `value`).
fn parse_setoption<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Command {
    // Expect "name" keyword
    match tokens.next() {
        Some("name") => {}
        _ => return Command::Unknown("setoption: expected 'name' keyword".to_string()),
    }

    // Collect name tokens until "value" keyword
    let mut name_parts = Vec::new();
    let mut value_parts = Vec::new();
    let mut found_value = false;

    for token in tokens {
        if token == "value" && !found_value {
            found_value = true;
            continue;
        }
        if found_value {
            value_parts.push(token);
        } else {
            name_parts.push(token);
        }
    }

    if name_parts.is_empty() {
        return Command::Unknown("setoption: missing option name".to_string());
    }

    Command::SetOption {
        name: name_parts.join(" "),
        value: value_parts.join(" "),
    }
}

/// Parse `position fen4 <fen> [moves <move_list>]`
/// or `position startpos [moves <move_list>]`.
fn parse_position<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Command {
    match tokens.next() {
        Some("startpos") => {
            let moves = parse_move_list(tokens);
            Command::PositionStartpos { moves }
        }
        Some("fen4") => {
            // Collect FEN tokens until "moves" keyword or end
            let mut fen_parts = Vec::new();
            let mut move_tokens = Vec::new();
            let mut found_moves = false;

            for token in tokens {
                if token == "moves" && !found_moves {
                    found_moves = true;
                    continue;
                }
                if found_moves {
                    move_tokens.push(token.to_string());
                } else {
                    fen_parts.push(token);
                }
            }

            if fen_parts.is_empty() {
                return Command::Unknown("position fen4: missing FEN4 string".to_string());
            }

            Command::PositionFen4 {
                fen: fen_parts.join(" "),
                moves: move_tokens,
            }
        }
        Some(other) => Command::Unknown(format!(
            "position: expected 'startpos' or 'fen4', got '{other}'"
        )),
        None => Command::Unknown("position: missing subcommand".to_string()),
    }
}

/// Parse optional `moves <move_list>` from remaining tokens.
fn parse_move_list<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Vec<String> {
    match tokens.next() {
        Some("moves") => tokens.map(|t| t.to_string()).collect(),
        Some(_) | None => Vec::new(),
    }
}

/// Parse `go` command with optional search limits.
fn parse_go<'a>(tokens: impl Iterator<Item = &'a str>) -> Command {
    let mut limits = SearchLimits::default();
    let token_vec: Vec<&str> = tokens.collect();
    let mut i = 0;

    while i < token_vec.len() {
        match token_vec[i] {
            "wtime" => {
                limits.wtime = parse_next_u64(&token_vec, &mut i);
            }
            "btime" => {
                limits.btime = parse_next_u64(&token_vec, &mut i);
            }
            "ytime" => {
                limits.ytime = parse_next_u64(&token_vec, &mut i);
            }
            "gtime" => {
                limits.gtime = parse_next_u64(&token_vec, &mut i);
            }
            "winc" => {
                limits.winc = parse_next_u64(&token_vec, &mut i);
            }
            "binc" => {
                limits.binc = parse_next_u64(&token_vec, &mut i);
            }
            "yinc" => {
                limits.yinc = parse_next_u64(&token_vec, &mut i);
            }
            "ginc" => {
                limits.ginc = parse_next_u64(&token_vec, &mut i);
            }
            "movestogo" => {
                limits.movestogo = parse_next_u64(&token_vec, &mut i).map(|v| v as u32);
            }
            "depth" => {
                limits.depth = parse_next_u64(&token_vec, &mut i).map(|v| v as u32);
            }
            "nodes" => {
                limits.nodes = parse_next_u64(&token_vec, &mut i);
            }
            "movetime" => {
                limits.movetime = parse_next_u64(&token_vec, &mut i);
            }
            "infinite" => {
                limits.infinite = true;
            }
            _ => {
                // Skip unknown tokens
            }
        }
        i += 1;
    }

    Command::Go(limits)
}

/// Parse the next token as u64, advancing the index.
fn parse_next_u64(tokens: &[&str], i: &mut usize) -> Option<u64> {
    *i += 1;
    tokens.get(*i).and_then(|t| t.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_odin() {
        assert_eq!(parse_command("odin"), Command::Odin);
        assert_eq!(parse_command("  odin  \n"), Command::Odin);
    }

    #[test]
    fn test_parse_isready() {
        assert_eq!(parse_command("isready"), Command::IsReady);
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(parse_command("quit"), Command::Quit);
    }

    #[test]
    fn test_parse_stop() {
        assert_eq!(parse_command("stop"), Command::Stop);
    }

    #[test]
    fn test_parse_setoption() {
        assert_eq!(
            parse_command("setoption name Debug value true"),
            Command::SetOption {
                name: "Debug".to_string(),
                value: "true".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_setoption_multi_word_name() {
        assert_eq!(
            parse_command("setoption name Clear Hash value"),
            Command::SetOption {
                name: "Clear Hash".to_string(),
                value: String::new(),
            }
        );
    }

    #[test]
    fn test_parse_setoption_no_value() {
        // setoption name X with no "value" keyword
        assert_eq!(
            parse_command("setoption name Terrain"),
            Command::SetOption {
                name: "Terrain".to_string(),
                value: String::new(),
            }
        );
    }

    #[test]
    fn test_parse_position_startpos() {
        assert_eq!(
            parse_command("position startpos"),
            Command::PositionStartpos { moves: vec![] }
        );
    }

    #[test]
    fn test_parse_position_startpos_with_moves() {
        assert_eq!(
            parse_command("position startpos moves d4d5 k8k9"),
            Command::PositionStartpos {
                moves: vec!["d4d5".to_string(), "k8k9".to_string()],
            }
        );
    }

    #[test]
    fn test_parse_position_fen4() {
        let fen = "some/fen4/string r ABCDabcd - 0 1";
        let cmd = parse_command(&format!("position fen4 {fen}"));
        assert_eq!(
            cmd,
            Command::PositionFen4 {
                fen: fen.to_string(),
                moves: vec![],
            }
        );
    }

    #[test]
    fn test_parse_position_fen4_with_moves() {
        let cmd = parse_command("position fen4 some/fen r ABcd - 0 1 moves d4d5 e7e8q");
        assert_eq!(
            cmd,
            Command::PositionFen4 {
                fen: "some/fen r ABcd - 0 1".to_string(),
                moves: vec!["d4d5".to_string(), "e7e8q".to_string()],
            }
        );
    }

    #[test]
    fn test_parse_go_with_times() {
        let cmd = parse_command("go wtime 60000 btime 55000 ytime 50000 gtime 45000");
        match cmd {
            Command::Go(limits) => {
                assert_eq!(limits.wtime, Some(60000));
                assert_eq!(limits.btime, Some(55000));
                assert_eq!(limits.ytime, Some(50000));
                assert_eq!(limits.gtime, Some(45000));
                assert!(!limits.infinite);
            }
            _ => panic!("expected Go command"),
        }
    }

    #[test]
    fn test_parse_go_with_depth() {
        let cmd = parse_command("go depth 8");
        match cmd {
            Command::Go(limits) => {
                assert_eq!(limits.depth, Some(8));
                assert_eq!(limits.wtime, None);
            }
            _ => panic!("expected Go command"),
        }
    }

    #[test]
    fn test_parse_go_with_movetime() {
        let cmd = parse_command("go movetime 5000");
        match cmd {
            Command::Go(limits) => {
                assert_eq!(limits.movetime, Some(5000));
            }
            _ => panic!("expected Go command"),
        }
    }

    #[test]
    fn test_parse_go_with_nodes() {
        let cmd = parse_command("go nodes 100000");
        match cmd {
            Command::Go(limits) => {
                assert_eq!(limits.nodes, Some(100000));
            }
            _ => panic!("expected Go command"),
        }
    }

    #[test]
    fn test_parse_go_infinite() {
        let cmd = parse_command("go infinite");
        match cmd {
            Command::Go(limits) => {
                assert!(limits.infinite);
            }
            _ => panic!("expected Go command"),
        }
    }

    #[test]
    fn test_parse_go_empty() {
        let cmd = parse_command("go");
        match cmd {
            Command::Go(limits) => {
                assert_eq!(limits, SearchLimits::default());
            }
            _ => panic!("expected Go command"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        match parse_command("foobar hello world") {
            Command::Unknown(s) => assert_eq!(s, "foobar hello world"),
            _ => panic!("expected Unknown"),
        }
    }

    #[test]
    fn test_parse_empty_input() {
        match parse_command("") {
            Command::Unknown(s) => assert!(s.is_empty()),
            _ => panic!("expected Unknown"),
        }
    }

    #[test]
    fn test_parse_whitespace_only() {
        match parse_command("   \t  \n  ") {
            Command::Unknown(s) => assert!(s.is_empty()),
            _ => panic!("expected Unknown"),
        }
    }

    #[test]
    fn test_parse_position_missing_subcommand() {
        match parse_command("position") {
            Command::Unknown(_) => {}
            _ => panic!("expected Unknown for missing subcommand"),
        }
    }

    #[test]
    fn test_parse_position_invalid_subcommand() {
        match parse_command("position garbage") {
            Command::Unknown(s) => assert!(s.contains("expected 'startpos' or 'fen4'")),
            _ => panic!("expected Unknown"),
        }
    }

    #[test]
    fn test_parse_setoption_missing_name() {
        match parse_command("setoption") {
            Command::Unknown(_) => {}
            _ => panic!("expected Unknown for missing name keyword"),
        }
    }

    #[test]
    fn test_parse_go_with_increments() {
        let cmd = parse_command(
            "go wtime 60000 winc 1000 btime 60000 binc 1000 ytime 60000 yinc 1000 gtime 60000 ginc 1000",
        );
        match cmd {
            Command::Go(limits) => {
                assert_eq!(limits.wtime, Some(60000));
                assert_eq!(limits.winc, Some(1000));
                assert_eq!(limits.btime, Some(60000));
                assert_eq!(limits.binc, Some(1000));
                assert_eq!(limits.ytime, Some(60000));
                assert_eq!(limits.yinc, Some(1000));
                assert_eq!(limits.gtime, Some(60000));
                assert_eq!(limits.ginc, Some(1000));
            }
            _ => panic!("expected Go command"),
        }
    }

    #[test]
    fn test_parse_go_with_movestogo() {
        let cmd = parse_command("go wtime 60000 btime 60000 ytime 60000 gtime 60000 movestogo 30");
        match cmd {
            Command::Go(limits) => {
                assert_eq!(limits.movestogo, Some(30));
                assert_eq!(limits.wtime, Some(60000));
            }
            _ => panic!("expected Go command"),
        }
    }
}
