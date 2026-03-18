/* AIOS HAL bare-metal llama inference shim.
 * When AIOS_LLAMA_LINKED: uses libllama.a for on-device inference.
 * Otherwise: stub returns -1 (use serial bridge for Ollama).
 *
 * Build: see hal-bare/build.rs
 */

#include <stddef.h>

#ifndef AIOS_LLAMA_LINKED

/* Stub when libllama not linked */
int aios_llama_inference(const char *prompt, char *out, size_t out_len) {
    (void)prompt;
    (void)out;
    (void)out_len;
    return -1;  /* Use bridge (simulate-rpi-bridge.sh) for Ollama. */
}

#else

/* Full implementation when libllama.a is linked */
#include "llama.h"
#include <string.h>
#include <stdbool.h>

#define MAX_PROMPT_TOKENS 256
#define MAX_PREDICT       64

static bool backend_inited;
static struct llama_model *model;
static struct llama_context *ctx;
static struct llama_sampler *smpl;

static void aios_llama_free(void);

/* Userdata for set_tensor_data when loading from buffer */
typedef struct {
    const void *buf;
    struct gguf_context *meta;
} aios_llama_buf_ud;

static void aios_llama_set_tensor_from_buf(struct ggml_tensor *tensor, void *userdata) {
    aios_llama_buf_ud *ud = (aios_llama_buf_ud *)userdata;
    if (!ud || !ud->buf || !ud->meta || !tensor || !tensor->data) return;
    int ti = gguf_find_tensor(ud->meta, ggml_get_name(tensor));
    if (ti < 0) return;
    size_t off = gguf_get_data_offset(ud->meta) + gguf_get_tensor_offset(ud->meta, ti);
    size_t n = ggml_nbytes(tensor);
    const char *src = (const char *)ud->buf + off;
    memcpy(tensor->data, src, n);
}

int aios_llama_init_from_memory(const void *buf, size_t len) {
    if (!buf || len < 4) return -1;
    if (!backend_inited) {
        llama_backend_init();
        backend_inited = true;
    }
    if (model || ctx) aios_llama_free();

    struct gguf_init_params gparams = { .no_alloc = true, .ctx = NULL };
    struct gguf_context *meta = gguf_init_from_buffer(buf, len, gparams);
    if (!meta) return -1;

    aios_llama_buf_ud ud = { .buf = buf, .meta = meta };
    struct llama_model_params mparams = llama_model_default_params();
    mparams.n_gpu_layers = -1;
    mparams.use_mmap = false;
    mparams.use_mlock = true;
    model = llama_model_init_from_user(meta, aios_llama_set_tensor_from_buf, &ud, mparams);
    gguf_free(meta);
    if (!model) return -1;

    struct llama_context_params cparams = llama_context_default_params();
    cparams.n_ctx = 512;
    cparams.n_batch = 256;
    cparams.n_threads = 1;
    ctx = llama_init_from_model(model, cparams);
    if (!ctx) {
        llama_model_free(model);
        model = NULL;
        return -1;
    }
    struct llama_sampler_chain_params sparams = llama_sampler_chain_default_params();
    smpl = llama_sampler_chain_init(sparams);
    llama_sampler_chain_add(smpl, llama_sampler_init_greedy());
    return 0;
}

/* Load from file path (requires _open/_read/_lseek syscall stubs to read from block). */
int aios_llama_init_from_file(const char *path) {
    if (!path || !*path) return -1;
    if (!backend_inited) {
        llama_backend_init();
        backend_inited = true;
    }
    if (model || ctx) aios_llama_free();
    struct llama_model_params mparams = llama_model_default_params();
    mparams.n_gpu_layers = -1;  /* CPU only on bare-metal */
    mparams.use_mmap = false;
    mparams.use_mlock = true;
    model = llama_model_load_from_file(path, mparams);
    if (!model) return -1;
    struct llama_context_params cparams = llama_context_default_params();
    cparams.n_ctx = 512;
    cparams.n_batch = 256;
    cparams.n_threads = 1;
    ctx = llama_init_from_model(model, cparams);
    if (!ctx) {
        llama_model_free(model);
        model = NULL;
        return -1;
    }
    struct llama_sampler_chain_params sparams = llama_sampler_chain_default_params();
    smpl = llama_sampler_chain_init(sparams);
    llama_sampler_chain_add(smpl, llama_sampler_init_greedy());
    return 0;
}

static int do_inference(const char *prompt, char *out, size_t out_len, int n_predict) {
    const struct llama_vocab *vocab = llama_model_get_vocab(model);
    llama_token prompt_tokens[MAX_PROMPT_TOKENS];
    int n_prompt = llama_tokenize(vocab, prompt, (int32_t)strlen(prompt),
                                  prompt_tokens, MAX_PROMPT_TOKENS, true, true);
    if (n_prompt <= 0 || n_prompt > MAX_PROMPT_TOKENS) return -1;

    struct llama_batch batch = llama_batch_get_one(prompt_tokens, n_prompt);

    if (llama_model_has_encoder(model)) {
        if (llama_encode(ctx, batch) != 0) return -1;
        llama_token dec_start = llama_model_decoder_start_token(model);
        if (dec_start == LLAMA_TOKEN_NULL) dec_start = llama_vocab_bos(vocab);
        batch = llama_batch_get_one(&dec_start, 1);
    }

    size_t written = 0;

    for (int i = 0; i < n_predict && written < out_len - 1; i++) {
        if (llama_decode(ctx, batch) != 0) return -1;

        llama_token tok = llama_sampler_sample(smpl, ctx, -1);
        if (llama_vocab_is_eog(vocab, tok)) break;

        int n = llama_token_to_piece(vocab, tok, out + written,
                                     (int32_t)(out_len - written - 1), 0, true);
        if (n < 0) break;
        written += (size_t)n;

        batch = llama_batch_get_one(&tok, 1);
    }

    if (written < out_len) out[written] = '\0';
    return (int)written;
}

int aios_llama_inference(const char *prompt, char *out, size_t out_len) {
    if (!prompt || !out || out_len == 0) return -1;

    if (!backend_inited) {
        llama_backend_init();
        backend_inited = true;
    }

    if (!model || !ctx || !smpl) return -1;

    return do_inference(prompt, out, out_len, MAX_PREDICT);
}

static void aios_llama_free(void) {
    if (smpl) {
        llama_sampler_free(smpl);
        smpl = NULL;
    }
    if (ctx) {
        llama_free(ctx);
        ctx = NULL;
    }
    if (model) {
        llama_model_free(model);
        model = NULL;
    }
    if (backend_inited) {
        llama_backend_free();
        backend_inited = false;
    }
}

#endif
