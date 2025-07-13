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

---

## 2. transport Layer

* **Trait definition** (blanket‑implemented over any `Read + Write + Send + 'static`):

  ```rust
  pub trait Transport: std::io::Read + std::io::Write + Send + 'static {}
  impl<T: std::io::Read + std::io::Write + Send + 'static> Transport for T {}
  ```

* **Concrete implementations**:

  * `std::net::TcpStream`
  * TLS streams via `native-tls` or `rustls`
  * In-memory mocks for unit tests

---

## 3. handshake Layer

Encapsulates the WebSocket HTTP‑Upgrade (RFC‑6455) handshake:

1. Build request:

   ```http
   GET /path HTTP/1.1

   Host: example.com

   Upgrade: websocket

   Connection: Upgrade

   Sec-WebSocket-Key: <nonce>

   Sec-WebSocket-Version: 13

   [extra headers]

   \r\n
   ```
2. Send via `stream.write_all(...)` + `flush()`
3. Read response until `\r\n\r\n`
4. Parse status 101 and headers via `httparse`
5. Validate `Sec-WebSocket-Accept` (SHA‑1 + Base64 of nonce + magic)
6. Return the same `Transport` on success

---

## 4. framing Layer

Handles packing/unpacking of WebSocket frames (RFC‑6455):

* **`Frame` struct**:

  ```rust
  pub struct Frame {
      pub fin: bool,
      pub opcode: OpCode,
      pub payload: Vec<u8>,
  }
  ```

* **Methods**:

  * `encode(&self, mask: Option<[u8;4]>) -> Vec<u8>`
  * `decode(buf: &[u8]) -> Result<(Self, usize), FrameDecodeError>`

* **`Framed<S>`** around `S: Transport`:

  ```rust
  pub struct Framed<S: Transport> {
      stream: S,
      read_buf: Vec<u8>,
  }
  impl<S: Transport> Framed<S> {
      pub fn send_frame(&mut self, frame: Frame) -> Result<(), WsError> { /* ... */ }
      pub fn next_frame(&mut self) -> Result<Frame, WsError> { /* ... */ }
  }
  ```

---

## 5. message API

High‑level user interface over frames:

* **`Message` enum**:

  ```rust
  pub enum Message {
      Text(String),
      Binary(Vec<u8>),
      Ping(Vec<u8>),
      Pong(Vec<u8>),
      Close(Option<CloseFrame>),
  }
  ```

* **`WebSocket<S>` struct**:

  ```rust
  pub struct WebSocket<S: Transport> {
      inner: Framed<S>,
  }
  impl<S: Transport> WebSocket<S> {
      pub fn send(&mut self, msg: Message) -> Result<(), WsError> { /* ... */ }
      pub fn receive(&mut self) -> Result<Message, WsError> { /* ... */ }
      pub fn close(&mut self, code: u16, reason: &str) -> Result<(), WsError> { /* ... */ }
  }
  ```

