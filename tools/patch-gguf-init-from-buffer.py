#!/usr/bin/env python3
"""Patch gguf.cpp to add gguf_init_from_buffer for loading GGUF from memory.
Run from project root. Modifies target/llama-build/llama.cpp/ggml/src/gguf.cpp
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GGUF_CPP = ROOT / "target" / "llama-build" / "llama.cpp" / "ggml" / "src" / "gguf.cpp"

def main():
    if not GGUF_CPP.exists():
        print(f"gguf.cpp not found: {GGUF_CPP}", file=sys.stderr)
        return 1
    c = GGUF_CPP.read_text()
    if "gguf_reader_buf" in c:
        print("gguf_init_from_buffer patch already applied")
        return 0
    # 1. Add gguf_reader_buf after gguf_reader (use + to avoid ''' inside triple-quoted string)
    old_reader_end = "private:\n    FILE * file;\n\n    mutable uint64_t nbytes_remain;\n};\n\nstruct gguf_context * gguf_init_empty"
    buf_reader_prefix = """private:
    FILE * file;

    mutable uint64_t nbytes_remain;
};

/* Buffer-based reader for gguf_init_from_buffer (no FILE* dependency) */
struct gguf_reader_buf {
    gguf_reader_buf(const char * data, size_t size) : data(data), size(size), offset(0) {}

    template <typename T>
    bool read(T & dst) const {
        const size_t n = sizeof(dst);
        if (offset + n > size) return false;
        memcpy(&dst, data + offset, n);
        offset += n;
        return true;
    }

    template <typename T>
    bool read(std::vector<T> & dst, const size_t n) const {
        if (n > GGUF_MAX_ARRAY_ELEMENTS) return false;
        if constexpr (std::is_same<T, std::string>::value) {
            if (n > SIZE_MAX / sizeof(uint64_t)) return false;
            if (offset + n * sizeof(uint64_t) > size) return false;
        } else {
            if (n > SIZE_MAX / sizeof(T)) return false;
            if (offset + n * sizeof(T) > size) return false;
        }
        dst.resize(n);
        for (size_t i = 0; i < dst.size(); ++i) {
            if constexpr (std::is_same<T, bool>::value) {
                bool tmp;
                if (!read(tmp)) return false;
                dst[i] = tmp;
            } else {
                if (!read(dst[i])) return false;
            }
        }
        return true;
    }

    bool read(bool & dst) const {
        int8_t tmp = -1;
        if (!read(tmp)) return false;
        dst = tmp != 0;
        return true;
    }

    bool read(enum ggml_type & dst) const {
        int32_t tmp = -1;
        if (!read(tmp)) return false;
        dst = ggml_type(tmp);
        return true;
    }

    bool read(enum gguf_type & dst) const {
        int32_t tmp = -1;
        if (!read(tmp)) return false;
        dst = gguf_type(tmp);
        return true;
    }

    bool read(std::string & dst) const {
        uint64_t len = 0;
        if (!read(len)) return false;
        if (len > GGUF_MAX_STRING_LENGTH) return false;
        if (offset + len > size) return false;
        dst.assign(data + offset, len);
        offset += len;
        return true;
    }

    bool read(void * dst, const size_t n) const {
        if (offset + n > size) return false;
        memcpy(dst, data + offset, n);
        offset += n;
        return true;
    }

    size_t tell() const { return offset; }
    void seek(size_t pos) const { offset = pos; }

private:
    const char * data;
    size_t size;
    mutable size_t offset;
};

struct gguf_context * gguf_init_empty"""
    buf_reader = buf_reader_prefix
    if old_reader_end not in c:
        print("Could not find gguf_reader end marker", file=sys.stderr)
        return 1
    c = c.replace(old_reader_end, buf_reader)
    # 2. Make gguf_read_emplace_helper generic
    c = c.replace(
        "template<typename T>\nbool gguf_read_emplace_helper(const struct gguf_reader & gr,",
        "template<typename T, typename Reader>\nbool gguf_read_emplace_helper(const Reader & gr,"
    )
    # 3. Add gguf_init_from_buffer after gguf_init_from_file
    old_file_end = (
        "struct gguf_context * result = gguf_init_from_file_impl(file, params);\n"
        "    fclose(file);\n"
        "    return result;\n"
        "}\n"
        "\n"
        "void gguf_free"
    )
    buf_init = '''struct gguf_context * result = gguf_init_from_file_impl(file, params);
    fclose(file);
    return result;
}

struct gguf_context * gguf_init_from_buffer(const void * buf, size_t buf_len, struct gguf_init_params params) {
    if (buf == nullptr || buf_len < 4) return nullptr;
    const char * data = static_cast<const char *>(buf);
    gguf_reader_buf gr(data, buf_len);
    struct gguf_context * ctx = new gguf_context;
    bool ok = true;
    std::vector<char> magic;
    ok = ok && gr.read(magic, 4);
    if (!ok) { GGML_LOG_ERROR("%s: failed to read magic\\n", __func__); gguf_free(ctx); return nullptr; }
    for (uint32_t i = 0; i < magic.size(); i++) {
        if (magic[i] != GGUF_MAGIC[i]) { GGML_LOG_ERROR("%s: invalid magic\\n", __func__); gguf_free(ctx); return nullptr; }
    }
    int64_t n_kv = 0, n_tensors = 0;
    if (!gr.read(ctx->version)) { gguf_free(ctx); return nullptr; }
    if (ctx->version == 0 || (ctx->version & 0x0000FFFF) == 0 || ctx->version == 1 || ctx->version > GGUF_VERSION) { gguf_free(ctx); return nullptr; }
    if (!gr.read(n_tensors) || n_tensors < 0 || n_tensors > int64_t(SIZE_MAX/sizeof(gguf_tensor_info))) { gguf_free(ctx); return nullptr; }
    if (!gr.read(n_kv) || n_kv < 0 || n_kv > int64_t(SIZE_MAX/sizeof(gguf_kv))) { gguf_free(ctx); return nullptr; }
    for (int64_t i = 0; ok && i < n_kv; ++i) {
        std::string key; gguf_type type = gguf_type(-1); bool is_array = false; uint64_t n = 1;
        if (!gr.read(key)) { ok = false; break; }
        if (!gr.read(type)) { ok = false; break; }
        if (type == GGUF_TYPE_ARRAY) { is_array = true; if (!gr.read(type) || !gr.read(n)) { ok = false; break; } }
        switch (type) {
            case GGUF_TYPE_UINT8:   ok = ok && gguf_read_emplace_helper<uint8_t   >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_INT8:    ok = ok && gguf_read_emplace_helper<int8_t    >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_UINT16:  ok = ok && gguf_read_emplace_helper<uint16_t  >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_INT16:   ok = ok && gguf_read_emplace_helper<int16_t   >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_UINT32:  ok = ok && gguf_read_emplace_helper<uint32_t  >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_INT32:   ok = ok && gguf_read_emplace_helper<int32_t   >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_FLOAT32: ok = ok && gguf_read_emplace_helper<float     >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_BOOL:    ok = ok && gguf_read_emplace_helper<bool      >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_STRING:  ok = ok && gguf_read_emplace_helper<std::string>(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_UINT64:  ok = ok && gguf_read_emplace_helper<uint64_t  >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_INT64:   ok = ok && gguf_read_emplace_helper<int64_t   >(gr, ctx->kv, key, is_array, n); break;
            case GGUF_TYPE_FLOAT64: ok = ok && gguf_read_emplace_helper<double    >(gr, ctx->kv, key, is_array, n); break;
            default: ok = false; break;
        }
    }
    if (!ok) { gguf_free(ctx); return nullptr; }
    const int alignment_idx = gguf_find_key(ctx, GGUF_KEY_GENERAL_ALIGNMENT);
    ctx->alignment = alignment_idx == -1 ? GGUF_DEFAULT_ALIGNMENT : gguf_get_val_u32(ctx, alignment_idx);
    if (ctx->alignment == 0 || (ctx->alignment & (ctx->alignment - 1)) != 0) { gguf_free(ctx); return nullptr; }
    for (int64_t i = 0; ok && i < n_tensors; ++i) {
        struct gguf_tensor_info info;
        std::string name; if (!gr.read(name)) { ok = false; break; }
        if (name.length() >= GGML_MAX_NAME) { ok = false; break; }
        ggml_set_name(&info.t, name.c_str());
        uint32_t n_dims = 0; if (!gr.read(n_dims) || n_dims > GGML_MAX_DIMS) { ok = false; break; }
        for (uint32_t j = 0; j < GGML_MAX_DIMS; ++j) { info.t.ne[j] = 1; if (j < n_dims && !gr.read(info.t.ne[j])) { ok = false; break; } }
        if (!gr.read(info.t.type)) { ok = false; break; }
        if (info.t.type < 0 || info.t.type >= GGML_TYPE_COUNT) { ok = false; break; }
        const size_t type_size = ggml_type_size(info.t.type);
        const int64_t blck_size = ggml_blck_size(info.t.type);
        if (blck_size == 0 || info.t.ne[0] % blck_size != 0) { ok = false; break; }
        info.t.nb[0] = type_size; info.t.nb[1] = info.t.nb[0]*(info.t.ne[0]/blck_size);
        for (int j = 2; j < GGML_MAX_DIMS; ++j) info.t.nb[j] = info.t.nb[j-1]*info.t.ne[j-1];
        if (!gr.read(info.offset)) { ok = false; break; }
        ctx->info.push_back(info);
    }
    if (!ok) { gguf_free(ctx); return nullptr; }
    gr.seek(GGML_PAD(gr.tell(), ctx->alignment));
    ctx->offset = gr.tell();
    ctx->size = 0;
    for (size_t i = 0; i < ctx->info.size(); ++i) {
        const gguf_tensor_info & ti = ctx->info[i];
        if (ti.offset != ctx->size) { gguf_free(ctx); return nullptr; }
        ctx->size += GGML_PAD(ggml_nbytes(&ti.t), ctx->alignment);
    }
    if (params.ctx != nullptr) {
        size_t mem_size = 0;
        if (params.no_alloc) {
            mem_size = n_tensors * ggml_tensor_overhead();
        } else {
            if ((n_tensors + 1) != 0 && SIZE_MAX / (n_tensors + 1) < ggml_tensor_overhead()) { gguf_free(ctx); return nullptr; }
            mem_size = (n_tensors + 1) * ggml_tensor_overhead();
            if (SIZE_MAX - mem_size < ctx->size) { gguf_free(ctx); return nullptr; }
            mem_size += ctx->size;
        }
        struct ggml_init_params pdata = { mem_size, nullptr, params.no_alloc };
        *params.ctx = ggml_init(pdata);
        if (!*params.ctx) { gguf_free(ctx); return nullptr; }
        struct ggml_context * ctx_data = *params.ctx;
        struct ggml_tensor * data = nullptr;
        if (!params.no_alloc) {
            data = ggml_new_tensor_1d(ctx_data, GGML_TYPE_I8, ctx->size);
            if (!data || !gr.read(data->data, ctx->size)) { ggml_free(ctx_data); gguf_free(ctx); return nullptr; }
            ctx->data = data->data;
        }
        ggml_set_no_alloc(ctx_data, true);
        for (size_t i = 0; i < ctx->info.size(); ++i) {
            const gguf_tensor_info & info = ctx->info[i];
            struct ggml_tensor * cur = ggml_new_tensor(ctx_data, info.t.type, GGML_MAX_DIMS, info.t.ne);
            if (!cur) { ggml_free(ctx_data); gguf_free(ctx); return nullptr; }
            ggml_set_name(cur, info.t.name);
            if (!params.no_alloc) cur->data = (char *)data->data + info.offset;
        }
        ggml_set_no_alloc(ctx_data, params.no_alloc);
    }
    return ctx;
}

void gguf_free'''
    if old_file_end not in c:
        print("Could not find gguf_init_from_file end marker", file=sys.stderr)
        return 1
    c = c.replace(old_file_end, buf_init)
    GGUF_CPP.write_text(c)

    # 4. Add gguf_get_tensor_struct for metadata-only loader (get tensor meta from gguf)
    if "gguf_get_tensor_struct" not in c:
        old_get = "enum ggml_type gguf_get_tensor_type(const struct gguf_context * ctx, int64_t tensor_id) {"
        new_get = (
            "struct ggml_tensor * gguf_get_tensor_struct(const struct gguf_context * ctx, int64_t tensor_id) {\n"
            "    if (tensor_id < 0 || tensor_id >= int64_t(ctx->info.size())) return nullptr;\n"
            "    return const_cast<struct ggml_tensor *>(&ctx->info[tensor_id].t);\n"
            "}\n\n"
            "enum ggml_type gguf_get_tensor_type(const struct gguf_context * ctx, int64_t tensor_id) {"
        )
        if old_get in c:
            c = c.replace(old_get, new_get)

    GGUF_CPP.write_text(c)

    # 5. Add gguf_init_from_buffer and gguf_get_tensor_struct declarations to gguf.h
    GGUF_H = ROOT / "target" / "llama-build" / "llama.cpp" / "ggml" / "include" / "gguf.h"
    if GGUF_H.exists():
        h = GGUF_H.read_text()
        if "//GGML_API struct gguf_context * gguf_init_from_buffer" in h:
            h = h.replace(
                "//GGML_API struct gguf_context * gguf_init_from_buffer(..);",
                "GGML_API struct gguf_context * gguf_init_from_buffer(const void * buf, size_t buf_len, struct gguf_init_params params);"
            )
        if "gguf_get_tensor_struct" not in h:
            h = h.replace(
                "GGML_API int64_t        gguf_find_tensor      (const struct gguf_context * ctx, const char * name);",
                "GGML_API int64_t        gguf_find_tensor      (const struct gguf_context * ctx, const char * name);\n    GGML_API struct ggml_tensor * gguf_get_tensor_struct(const struct gguf_context * ctx, int64_t tensor_id);"
            )
        GGUF_H.write_text(h)
        print("Updated gguf.h")

    # 6. Patch llama-model-loader: get_tensor_meta fallback when weights_map empty (for init_from_user with buffer)
    LOADER_CPP = ROOT / "target" / "llama-build" / "llama.cpp" / "src" / "llama-model-loader.cpp"
    if LOADER_CPP.exists():
        loader = LOADER_CPP.read_text()
        if "gguf_get_tensor_struct" not in loader or "get_weight(name)" not in loader.replace("gguf_get_tensor_struct", ""):
            old_gfm = (
                "struct ggml_tensor * llama_model_loader::get_tensor_meta(const char * name) const {\n"
                "    const auto * weight = get_weight(name);\n"
                "    if (!weight) {\n"
                "        return nullptr;\n"
                "    }\n"
                "    return weight->tensor;\n"
                "}"
            )
            new_gfm = (
                "struct ggml_tensor * llama_model_loader::get_tensor_meta(const char * name) const {\n"
                "    const auto * weight = get_weight(name);\n"
                "    if (weight) {\n"
                "        return weight->tensor;\n"
                "    }\n"
                "    if (metadata && weights_map.empty()) {\n"
                "        const int64_t ti = gguf_find_tensor(metadata, name);\n"
                "        if (ti >= 0) return gguf_get_tensor_struct(metadata, ti);\n"
                "    }\n"
                "    return nullptr;\n"
                "}"
            )
            if old_gfm in loader:
                loader = loader.replace(old_gfm, new_gfm)
                old_loop = (
                    "        for (const auto & it : weights_map) {\n"
                    "            const llama_tensor_weight & w = it.second;\n"
                    "            const ggml_tensor * tensor = w.tensor;\n"
                    "            enum ggml_type type = tensor->type;\n"
                    "            n_type[type]++;"
                )
                new_loop = (
                    "        if (weights_map.empty() && metadata) {\n"
                    "            for (int64_t ti = 0; ti < gguf_get_n_tensors(metadata); ++ti) {\n"
                    "                struct ggml_tensor * tensor = gguf_get_tensor_struct(metadata, ti);\n"
                    "                if (tensor) { enum ggml_type type = tensor->type; n_type[type]++; if (n_type[type] > n_type_max) { n_type_max = n_type[type]; type_max = type; } }\n"
                    "            }\n"
                    "        }\n"
                    "        for (const auto & it : weights_map) {\n"
                    "            const llama_tensor_weight & w = it.second;\n"
                    "            const ggml_tensor * tensor = w.tensor;\n"
                    "            enum ggml_type type = tensor->type;\n"
                    "            n_type[type]++;"
                )
                if old_loop in loader and "weights_map.empty() && metadata" not in loader:
                    loader = loader.replace(old_loop, new_loop)
                loader = loader.replace(
                    "n_tensors = weights_map.size();",
                    "n_tensors = weights_map.empty() ? (int)gguf_get_n_tensors(metadata) : (int)weights_map.size();"
                )
                LOADER_CPP.write_text(loader)
                print("Patched llama-model-loader for buffer loading")
            elif new_gfm not in loader:
                print("Warning: could not patch get_tensor_meta (pattern not found)", file=sys.stderr)

    print("Applied gguf_init_from_buffer patch")
    return 0

if __name__ == "__main__":
    sys.exit(main())
