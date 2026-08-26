# 🤝 Contributing to Aura Network & Blockchain

Welcome to the **Aura Network** open-source family! Whether you are an experienced systems programmer, a cryptography researcher, a frontend developer, or writing your very first pull request, you are welcome here.

---

## 🌟 How Anyone Can Join Our Code Building Journey

Aura is built as a modular, high-performance, and decentralized ecosystem consisting of:
1. **Low-Level Systems Engineering (Rust):** Cryptography, PoS-BFT Consensus, Sparse Merkle Trees, Mempool, Networking, P2P Tunnels.
2. **Modern Web Engineering (TypeScript/Next.js/Tailwind):** Block Explorer, Non-Custodial Web Wallet, Mesh dVPN UI.
3. **Protocol & Research:** Slashing models, Zero-Knowledge proofs, Carrier NAT bypass protocols, Tokenomics.

---

## 🚀 Getting Started

### 1. Fork and Clone
```bash
git clone https://github.com/viroaryan/Aura-Blockchain.git
cd Aura-Blockchain
```

### 2. Prerequisites
- **Rust Toolchain:** `rustup default stable` (1.75+)
- **Node.js:** v18.0+ or v20+ with `npm`

### 3. Build & Run Locally
```bash
# Build all Rust Crates
cargo build --workspace

# Run All Automated Test Suites
cargo test --workspace

# Run the Node Daemon
cargo run --bin aura-node -- run --data-dir ./.aura-data

# Run the Next.js Explorer & dVPN UI
cd explorer
npm install
npm run dev
```

---

## 🧭 Contribution Roadmap & Good First Issues

- [ ] **Core Crates (`crates/aura-*`):** Add EVM / WASM smart contract execution engine.
- [ ] **Mesh Tunnel (`crates/aura-tunnel`):** Multi-hop Onion Routing for enhanced privacy.
- [ ] **Mobile SDK:** Kotlin (Android) and Swift (iOS) native background VpnService bindings.
- [ ] **Explorer UI (`explorer/`):** Dark/Light mode toggle, multi-language localization (Hindi, Spanish, Japanese).

---

## 📜 Pull Request Guidelines

1. **Create a branch:** `git checkout -b feature/your-feature-name`
2. **Write clear commit messages:** Follow Conventional Commits (`feat: ...`, `fix: ...`, `docs: ...`).
3. **Ensure tests pass:** Run `cargo test --workspace` and `npm run build` in `explorer/`.
4. **Submit your PR:** Describe your changes clearly and link any relevant issues!

---

## 💬 Community & Communication
- **GitHub Discussions:** Ask questions, propose RFCs, or share ideas in the repository discussions.
- **Code of Conduct:** Please review our [Code of Conduct](./CODE_OF_CONDUCT.md) to ensure a welcoming environment for everyone.
