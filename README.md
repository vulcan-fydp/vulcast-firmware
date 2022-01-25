# vulcast-firmware
Requires [cargo-deb](https://github.com/mmstick/cargo-deb)

```
$ cargo build
$ cargo deb
$ sudo dpkg -i target/debian/*.deb
$ sudo systemctl status vulcast-firmware.service
```

## Cross-compile w/ Docker
### SSH setup
Some setup is required to clone private repositories from within the Docker container.
1. Generate an SSH key if you don't have one already.
2. Add your SSH key to GitHub.
3. Install and run [ssh-agent](https://wiki.archlinux.org/title/SSH_keys#ssh-agent).
4. Add your SSH private key to the ssh-agent cache

### Build cross-compile image
This step will build a Docker image that you can use to cross-compile this project. 
You can repeatedly use this image without needing to rebuild it, unless the Dockerfile changes.
```bash
docker build . -t vulcast-cross/armv7
```

### Build binary
This step will build the vulcast-firmware binary for the cross-compile target using the Docker image.
```bash
docker run --rm -ti \
	-v $(pwd):/app \
	-v $(readlink -f $SSH_AUTH_SOCK):/ssh-agent \
	-e SSH_AUTH_SOCK=/ssh-agent \
	vulcast-cross/armv7
```

The resultant binary will be in `target/armv7-unknown-linux/release`.
