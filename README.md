````markdown
# Rust WebSocket Client

> **⚠️ Still Work in Progress** – API may change as the project evolves.

A lightweight, synchronous WebSocket client library in Rust, built for low-latency trading and real-time applications.  
This crate implements its own TCP/TLS/WebSocket handshake logic and **manual polling state machine** — giving you full control over connection establishment and I/O flow.

---

## ✨ Features

- **Custom handshake** – Performs the WebSocket opening handshake over TCP (or TLS for `wss://`).
- **Frame encoding/decoding** – Build and parse WebSocket frames (`Text`, `Binary`, `Ping`, `Pong`, `Close`).
- **Text & binary messaging** – Send and receive `String` or `Vec<u8>` payloads via the `Message` type.
- **Two connection modes**:
  - **Blocking** – Use `connect_blocking()` for a one-call, ready-to-use WebSocket.
  - **Manual / Non-blocking** – Use `connect()` + `poll_once()` to manually drive TCP, TLS, and WS handshakes.
- **Manual polling design** – Underlying TCP stream is set to non-blocking mode; `poll_once()` and `recv()` return immediately when no data is available.
- **Error handling** – Strongly-typed errors via [`thiserror`](https://docs.rs/thiserror).

---

## 📦 Example Usage

### Blocking Mode
This mode hides the handshake state machine from the user and returns a ready `WebSocket`.

```rust
use std::str::FromStr;
use log::info;
use url::Url;
use ws_client::{connect_blocking, StreamError};

fn main() -> Result<(), StreamError> {
    env_logger::init();

    let url = Url::from_str("wss://fstream.binance.com/ws/bnbusdt@aggTrade")
        .expect("invalid URL");

    // Establish TCP, TLS (if wss), and WebSocket handshakes
    let mut ws = connect_blocking(url)?;

    loop {
        if let Some(msg) = ws.recv()? {
            info!("Received: {:?}", msg);
        }
    }
}
````

---

### Manual / Non-blocking Mode (Full Control)

This mode exposes the internal handshake process via `poll_once()`.
You can integrate it into your own event loop or readiness system.

```rust
use std::{str::FromStr, time::Duration};
use log::info;
use url::Url;
use ws_client::{connect, StreamError};

fn main() -> Result<(), StreamError> {
    env_logger::init();

    let url = Url::from_str("wss://fstream.binance.com/ws/bnbusdt@aggTrade")
        .expect("invalid URL");

    // Step 1: Create connection client
    let mut client = connect(url)?;

    // Step 2: Drive the handshake state machine manually
    let mut ws = loop {
        match client.poll_once() {
            Some(Ok(ws)) => break ws,       // Connected
            Some(Err(e)) => return Err(e),  // Failed
            None => {
                // No progress yet; in production replace with poll/select/epoll/kqueue
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    };

    // Step 3: Receive messages
    loop {
        match ws.recv() {
            Ok(Some(msg)) => info!("Received: {:?}", msg),
            Ok(None) => continue, // WouldBlock
            Err(e) => {
                eprintln!("WebSocket error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
```

---

## 📂 Crate Contents

* **`client`** – `ConnectionClient` and `WebSocket` high-level API, plus `connect` and `connect_blocking` entry points.
* **`handshake`** – `HandshakeClient` performs the WebSocket opening handshake.
* **`frame`** – `Frame` and `OpCode` for low-level WebSocket framing.
* **`transport`** – `Framed<S: Transport>` wraps a stream for frame-based I/O.
* **`message`** – `Message` enum for text/binary payloads.
* **`error`** – `StreamError` and related error definitions.

---

## 🚧 TODO

* Replace `std::thread::sleep` in handshake loops with proper socket readiness (`poll`, `epoll`, `kqueue`, or `mio`).
* Add optional permessage-deflate compression.
* Improve control frame handling (Ping/Pong/Close).
* Expand handshake to support custom HTTP headers.
* Add fuzz tests for frame parser.

---

## 🛠️ Manual Polling Design

This library is built around a **manual polling state machine**.
Instead of relying on async runtimes or blocking I/O during handshake, it:

1. Opens a TCP connection in **non-blocking** mode.
2. Optionally performs a TLS handshake (for `wss://`) in **non-blocking** steps.
3. Performs the WebSocket handshake incrementally.
4. Returns `Some(Ok(WebSocket))` when ready, or `Some(Err(...))` on failure, or `None` if more polling is needed.

This design makes it easy to:

* Integrate into existing single-threaded event loops.
* Use platform-specific I/O readiness APIs.
* Avoid unexpected blocking during connection setup.

```
```
