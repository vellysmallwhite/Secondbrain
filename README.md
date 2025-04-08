# SecondBrian - Secure Local Diary with Graph Visualization

SecondBrian is a secure, local-first diary application that uses graph visualization to help you explore connections between your thoughts and ideas.

## Features

- **Local-Only Storage**: All your data is stored locally on your device, with no internet connection required.
- **Military-Grade Encryption**: Uses AES-256-GCM encryption to protect your diary entries.
- **Graph Visualization**: Visualize connections between diary entries and tags.
- **Tag-Based Organization**: Organize your entries with tags and explore related content.
- **Cross-Platform**: Works on Windows, macOS, and Linux.

## Security

SecondBrian takes your privacy seriously:

- All diary content is encrypted using AES-256-GCM encryption
- Encryption keys are securely stored in your system's keychain
- No data is ever sent to any server
- All processing happens locally on your device

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) (v16 or later)
- [Rust](https://www.rust-lang.org/) (latest stable)
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

### Setup

1. Clone the repository
2. Install dependencies:
   ```
   npm install
   ```
3. Run the development server:
   ```
   npm run tauri dev
   ```

### Building

To build the application for production:

```
npm run tauri build
```

## License

MIT

## Credits

- Built with [Tauri](https://tauri.app/) and [Svelte](https://svelte.dev/)
- Graph visualization powered by [vis.js](https://visjs.org/)
