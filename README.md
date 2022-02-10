# vulcast-firmware
Requires [cargo-deb](https://github.com/mmstick/cargo-deb)

```
$ cargo build
$ cargo deb
$ sudo dpkg -i target/debian/*.deb
$ sudo systemctl status vulcast-firmware.service
```

## Cross-compile w/ Docker
### 1. SSH setup
Some setup is required to clone private repositories from within the Docker container.
1. [Generate an SSH key and add it to
ssh-agent](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/generating-a-new-ssh-key-and-adding-it-to-the-ssh-agent).
If you already have an SSH key, skip the generation step and just add it to
ssh-agent.
2. [Add the SSH key to your GitHub
account](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/adding-a-new-ssh-key-to-your-github-account)
if it is not added already.

### 2. Build cross-compile image
This step will build a Docker image that you can use to cross-compile this project. 
You can repeatedly use this image without needing to rebuild it, unless the Dockerfile changes.
```bash
docker build . -t vulcast-cross/armv7
```

### 3. Build binary
This step will build the vulcast-firmware binary for the cross-compile target using the Docker image.
```bash
docker run --rm -ti \
	-v $(pwd):/app \
	-v $(readlink -f $SSH_AUTH_SOCK):/ssh-agent \
	-e SSH_AUTH_SOCK=/ssh-agent \
	vulcast-cross/armv7
```

The resultant binary will be in `target/armv7-unknown-linux-gnueabihf/release`.
