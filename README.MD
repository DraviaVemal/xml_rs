# draviavemal-xml_rs (AGPL-3.0)

A high-performance, low-memory XML DOM parser for Rust — with **namespace-aware** parsing, **order-preserving attributes**, and **round-trip safe** serialization.

[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://crates.io/crates/draviavemal-xml_rs)
[![License](https://img.shields.io/badge/license-AGPL--3.0-green.svg)](LICENSE)

### 📜 License
- Open Source: AGPL-3.0 (see LICENSE)
- Commercial: Available via sponsorship (see LICENSE)

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