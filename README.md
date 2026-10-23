# Nefrita Desktop

Desktop app for Nefrita, an open source hardware wallet for Bitcoin BLAKE2b (XBT) that runs on DIY ESP32 boards.

> **Status: early development.** Nothing here is ready to hold funds.

## What it will do

- Connect to a Nefrita device over USB and unlock it with its PIN, through the Nefrita PIN server or your own.
- Show the balance and transactions of the wallet.
- Receive, verifying each address on the device screen.
- Send, with every signature confirmed on the device and protected against replay on Bitcoin.

The app is built with Rust, Tauri and React with TypeScript.

## Related projects

- [Nefrita firmware](https://github.com/Sharmaz/Jade), based on [Blockstream Jade](https://github.com/Blockstream/Jade). Nefrita is not affiliated with Blockstream.
- [The Nefrita organization](https://github.com/nefrita).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE).
