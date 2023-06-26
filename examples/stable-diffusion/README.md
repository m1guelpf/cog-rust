# Stable Diffusion

This is an implementation of the Diffusers Stable Diffusion v2.1 as a Cog model.

---

Download the pre-trained weights:

```
curl -L -o weights/bpe_simple_vocab_16e6.txt https://huggingface.co/lmz/rust-stable-diffusion-v2-1/raw/main/weights/bpe_simple_vocab_16e6.txt
curl -L -o weights/clip_v2.1.safetensors https://huggingface.co/lmz/rust-stable-diffusion-v2-1/resolve/main/weights/clip_v2.1.safetensors
curl -L -o weights/unet_v2.1.safetensors https://huggingface.co/lmz/rust-stable-diffusion-v2-1/resolve/main/weights/unet_v2.1.safetensors
curl -L -o weights/vae_v2.1.safetensors https://huggingface.co/lmz/rust-stable-diffusion-v2-1/resolve/main/weights/vae_v2.1.safetensors
```

Build the image:

```sh
cargo cog build
```

Now you can run predictions on the model:

```sh
cargo cog predict -i "prompt=a photo of a cat"
```
