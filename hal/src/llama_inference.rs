//! llama.cpp-backed inference via llama-cpp-2
//!
//! Requires `--features llama`. Uses GGUF models.

use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::context::params::LlamaContextParams;
use std::num::NonZeroU32;
use std::path::Path;

const MAX_TOKENS: i32 = 256;
const DEFAULT_CTX: u32 = 2048;

/// Run inference: prompt (UTF-8) -> generated text (UTF-8 bytes).
pub fn run_inference(model_path: &str, prompt: &str) -> Result<Vec<u8>, String> {
    let path = Path::new(model_path);
    if !path.exists() {
        return Err(format!("model not found: {}", model_path));
    }

    let backend = LlamaBackend::init().map_err(|e| e.to_string())?;
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, path, &model_params)
        .map_err(|e| e.to_string())?;

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(DEFAULT_CTX).unwrap());
    let mut ctx = model.new_context(&backend, ctx_params).map_err(|e| e.to_string())?;

    let tokens = model.str_to_token(prompt, AddBos::Always).map_err(|e| e.to_string())?;
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let n_len = (tokens.len() as i32) + MAX_TOKENS;
    let n_ctx = ctx.n_ctx() as i32;
    if n_len > n_ctx {
        return Err(format!("context too small: need {} have {}", n_len, n_ctx));
    }

    let mut batch = LlamaBatch::new(512, 1);
    let last_idx = (tokens.len() - 1) as i32;
    for (i, t) in tokens.into_iter().enumerate() {
        let is_last = i as i32 == last_idx;
        batch.add(t, i as i32, &[0], is_last).map_err(|e| e.to_string())?;
    }
    ctx.decode(&mut batch).map_err(|e| e.to_string())?;

    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::dist(1234),
        LlamaSampler::greedy(),
    ]);

    let mut output = Vec::new();
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut n_cur = batch.n_tokens();

    while n_cur < n_len {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if model.is_eog_token(token) {
            break;
        }

        let piece = model.token_to_piece(token, &mut decoder, true, None).map_err(|e| e.to_string())?;
        output.extend_from_slice(piece.as_bytes());

        batch.clear();
        batch.add(token, n_cur, &[0], true).map_err(|e| e.to_string())?;
        ctx.decode(&mut batch).map_err(|e| e.to_string())?;

        n_cur += 1;
    }

    Ok(output)
}
