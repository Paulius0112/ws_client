# WebSocket Client Library Design

This document outlines the high-level design and architecture for a synchronous WebSocket client library in Rust.

---

## 1. Overview

The library is composed of four main layers, each with a clear responsibility. The public API (`ClientBuilder`) ties these layers together to provide a simple `connect` + `send`/`receive` interface:

```
┌──────────────┐
│   client     │  ← Public API: `ClientBuilder::connect()`
└──────────────┘
        ↓
┌──────────────┐
│  transport   │  ← Trait + impls for byte I/O (TCP, TLS, mocks)
└──────────────┘
        ↓
┌──────────────┐
│ handshake    │  ← RFC‑6455 HTTP‑upgrade request/response
└──────────────┘
        ↓
┌──────────────┐
│ framing      │  ← WebSocket frame encode/decode (opcodes, masks)
└──────────────┘
        ↓
┌──────────────┐
│ message      │  ← High‑level `Message` enum and send/receive API
└──────────────┘
```