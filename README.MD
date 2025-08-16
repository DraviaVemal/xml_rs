# draviavemal-xml_rs (AGPL-3.0)

A high-performance, low-memory XML DOM parser for Rust — with **namespace-aware** parsing, **order-preserving attributes**, and **round-trip safe** serialization.

### 📜 License
- Open Source: AGPL-3.0 (see LICENSE)
- Commercial: Available via sponsorship (see LICENSE)

## ✨ Target Features

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

## 📦 Installation

`cargo add draviavemal-xml_rs`