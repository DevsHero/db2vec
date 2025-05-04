# TEI Provider (Text Embeddings Inference)

This project ships two TEI binaries under the `tei/` folder, built from **v1.7.0**:

- `tei/tei-metal`  – for Apple Silicon (M1/M2) using the Metal backend  
- `tei/tei-onnx`   – for x86_64 using the ONNX Runtime backend

Feel free to build your own from source:

```bash
git clone https://github.com/huggingface/text-embeddings-inference.git
cd text-embeddings-inference

# On x86_64 with ONNX backend (recommended)
cargo install --path router -F ort

# On x86_64 with Intel MKL
cargo install --path router -F mkl

# On Apple Silicon (M1/M2) with Metal
cargo install --path router -F metal
```

You can also run the TEI router standalone:

```bash
# e.g. on CPU:
text-embeddings-router --model-id YOUR_MODEL_ID --port 8080
```

> Note: on Linux you may need OpenSSL & gcc:
> `sudo apt-get install libssl-dev gcc -y`

---

## Using TEI with db2vec

When running `db2vec`, point it at one of the shipped binaries (or your own):

```bash
cargo run --release -- \
  -f your_dataset.surql \
  -t pinecone \
  --embedding-provider tei \
  --tei-binary-path tei/tei-metal \
  --embedding-model nomic-ai/nomic-embed-text-v2-moe \
  --dimension 768
```

- `--tei-binary-path` : path to `tei-metal` or `tei-onnx`  
- `--embedding-provider=tei` : select the TEI backend  
- `--embedding-model` : any HF model ID (e.g. `BAAI/bge-large-en-v1.5`)  
- `--dimension` : vector dimension (must match the model)

If you omit `--tei-binary-path`, db2vec will unpack its embedded copy (same binaries) into your temp directory and launch that automatically.

---

Happy vectorizing!  