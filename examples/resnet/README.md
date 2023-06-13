# resnet

This model classifies images.

---

Download the pre-trained weights:

```
curl -L -o weights/model.safetensors https://huggingface.co/microsoft/resnet-50/resolve/refs%2Fpr%2F4/model.safetensors
```

Build the image:

```sh
cargo cog build
```

Now you can run predictions on the model:

```sh
cargo cog predict -i image=@cat.png
```
