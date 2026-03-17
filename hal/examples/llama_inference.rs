//! Run: cargo run -p aios-hal --features llama --example llama_inference -- /path/to/model.gguf "Hello"
//!
//! Download a small GGUF model first, e.g.:
//! curl -L -o tinyllama.gguf https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <model.gguf> <prompt>", args.get(0).unwrap_or(&"llama_inference".into()));
        eprintln!("Example: {} ./tinyllama.gguf \"What is AI?\"", args.get(0).unwrap_or(&"llama_inference".into()));
        std::process::exit(1);
    }
    let model_path = &args[1];
    let prompt = &args[2];

    println!("Model: {}", model_path);
    println!("Prompt: {}", prompt);
    println!("---");

    match aios_hal::inference(model_path, prompt.as_bytes()) {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out);
            println!("{}", s);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
