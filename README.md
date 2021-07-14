# vulcast-firmware
Requires [cargo-deb](https://github.com/mmstick/cargo-deb)

```
$ cargo build
$ cargo deb
$ sudo dpkg -i target/debian/*.deb
$ sudo systemctl status vulcan-relay.service
```
