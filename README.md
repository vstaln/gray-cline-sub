<p align="center">
  <img src="assets/gray-logo.svg" alt="gray" width="96">
</p>
<h1 align="center">gray-cline-sub</h1>
<p align="center">Run gray turns on the Cline CLI — a Cline-subscription provider sidecar.</p>

Relays gray chat turns through the Cline CLI's `cline --acp` subprocess (ACP over stdio).
The vendor CLI does the talking; the sidecar is a model, not an agent runtime.

## Requirements

- Cline CLI (`cline`) on `PATH`, or set `GRAY_CLINE_SUB_COMMAND`.
- Signed in: `cline auth`.

## Setup

1. Install the plugin and enable it.
2. Run `/cline` (or `/cline status`) to check status: signed-in account and CLI version.
3. Models are discovered via an ACP probe. If discovery fails, set `GRAY_CLINE_SUB_MODELS`.

Auth method: `cline-login` (OAuth kind). Credentials stay with the Cline CLI.

## Environment

| Variable | Default | Meaning |
| --- | --- | --- |
| `GRAY_CLINE_SUB_COMMAND` | `cline` on `PATH` | Binary override |
| `GRAY_CLINE_SUB_MODE` | `act` | `plan` (read-only Q&A) or `act` |
| `GRAY_CLINE_SUB_MODELS` | unset | Static model list override for discovery failures |
| `GRAY_CLINE_SUB_DATA_DIR` | CLI default | Cline data directory override |

## License

MIT — see [LICENSE](LICENSE).
