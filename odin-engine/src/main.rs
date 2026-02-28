use odin_engine::protocol::OdinEngine;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Datagen subcommand: --datagen --input <file.jsonl> --output <file.bin>
    if args.iter().any(|a| a == "--datagen") {
        if let Err(e) = odin_engine::datagen::run(&args) {
            eprintln!("datagen error: {e}");
            std::process::exit(1);
        }
        return;
    }

    let mut engine = OdinEngine::new();
    engine.run();
}
