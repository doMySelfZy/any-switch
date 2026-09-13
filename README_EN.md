<p align="left">
  <a href="./README.md">中文</a> · <strong>English</strong>
</p>

<p align="center">
  <img src="assets/brand/app-icon-1024.png" alt="XiaoBaiSwitch Plus" width="160" height="160">
</p>

# XiaoBaiSwitch Plus

> **This project is a fork of [XiaoBaiSwitch](https://github.com/Licoy/xiaobai-switch) by [Licoy](https://github.com/Licoy).**
> The original project, authorship, and MIT license belong to the original author. This repository only adds enhancements (real sync, etc.) and publishes downloads and auto-updates from our own GitHub.

## Download & Update

- **Downloads and auto-updates come only from GitHub Releases**: <https://github.com/doMySelfZy/xiaobai-switch-plus/releases>
- The updater endpoint is that release's `latest.json`.
- **There is no website, and nothing is published on Gitee.**

## Features (brief)

- Site-first: Base URL + API key → models → target presets → apply to targets.
- Targets: Claude Code, Codex, Pi, and Prime, each with its own form.
- Local backups + real WebDAV sync: one dataset across machines, synced on change, pulled on open.
- `xiaobaiswitchplus://` deep links import a site in one click; legacy `anyswitch://` and `xiaobaiswitch://` links still work.
- API keys are encrypted at rest, configs are backed up before apply, and official configs can be restored.

See the [upstream project](https://github.com/Licoy/xiaobai-switch) for full documentation.

## Data directory

- Current directory: `~/.xiaobai-switch/` (`xiaobai-switch.db`, `master.key`, `backups/`).
- When upgrading from an AnySwitch build, the first launch **automatically takes over** the data in `~/.any-switch/`: it compares database modification times, copies the newer dataset, verifies it, then uses the new directory.
- The old `~/.any-switch/` directory is **kept as-is** as a rollback point and is never deleted; remove it yourself once you are confident.

## Development

```bash
pnpm install
pnpm tauri dev        # dev run
pnpm typecheck        # type check
pnpm test:run         # frontend tests
cd src-tauri && cargo test   # Rust tests
pnpm tauri build      # bundle
```

## License

MIT. Original copyright, authorship, and attribution belong to the upstream [Licoy/xiaobai-switch](https://github.com/Licoy/xiaobai-switch).
