# draviavemal-xml_rs (AGPL-3.0)

A high-performance, low-memory XML DOM parser for Rust — with **namespace-aware** parsing, **order-preserving attributes**, and **round-trip safe** serialization.

[![Latest Version](https://img.shields.io/crates/v/draviavemal-xml_rs)](https://crates.io/crates/draviavemal-xml_rs)
[![License](https://img.shields.io/badge/license-AGPL--3.0-green.svg)](LICENSE)

### 📜 License

This project is **dual-licensed**:

- **Open Source — AGPL-3.0** (see [LICENSE](LICENSE)). Free to use, including
  commercially, as long as you comply with AGPL-3.0 copyleft (share source of
  derivatives and network-served versions).
- **Commercial License** for closed-source / proprietary use without AGPL
  obligations — available to sponsors (see [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md)).

💜 If this project helps you or your business, please consider
[sponsoring](https://github.com/sponsors/DraviaVemal) — sponsors get the
commercial license, private releases, and priority support.

## ✨ Features

- **UTF-8 focused** XML parsing
- Reads **XML declaration** (version, encoding, standalone)
- **Line break normalization** (`\r\n` / `\r` → `\n`)
- Full **namespace resolution** with prefix → URI stack
- Correct **QName parsing** (`{namespace_uri, local_name}`)
- **Order-preserving attributes** (using `IndexMap`)
- Support for:
  - Elements (`<tag>` / `<tag/>`)
  - Attributes (namespaced & unnamespaced)
  - Mixed content (text + elements)
  - Comments (`<!-- -->`)
- **Round-trip safe** serialization:
  - Preserves original namespace prefixes
  - namespace alias rename
  - Preserves attribute order
- **Low memory** ID-based tree model (similar to arena allocation)
- No unsafe code

---

## 📖 Documentation
- Working on setting up a central system for all my OSS contribution will update link in description once ready. Please refer to tests crate present in this repository to understand the methods

## 📦 Installation

`cargo add draviavemal-xml_rs`