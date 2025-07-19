# Rust WebSocket Client

# ! Still work in Progress ! 

A lightweight, synchronous WebSocket client library in Rust. This crate provides a simple API for connecting to a WebSocket server, performing the handshake, sending and receiving text or binary messages, and handling frame encoding and decoding.

## Features

* **Custom handshake**: Implements the WebSocket opening handshake over a raw TCP stream.
* **Frame encoding/decoding**: Build and parse WebSocket frames (`Text`, `Binary`, `Ping`, `Pong`, `Close`).
* **Text and binary messaging**: Send and receive `String` or `Vec<u8>` payloads via the `Message` type.
* **Configurable connection**: Set timeouts, custom HTTP headers, and support for both `ws://` and `wss://` schemes.
* **Non-blocking I/O**: Underlying TCP stream is set to non-blocking mode; `recv` returns immediately when no data is available.
* **Error handling**: Uses `thiserror` for ergonomic error types.

## TODO
* Handshake response validation (header parsing etc).
* Support additional message types (besides String/Binary).
* Implement support for **wss** streams.
* Make the code fully non-blocking.

## Crate Contents

* **client**: `WebSocket<T: Transport>`—high-level API to send and receive `Message` values.
* **handshake**: `HandshakeClient`—performs the WebSocket opening handshake.
* **frame**: `Frame` and `OpCode`—low-level WebSocket framing utilities.
* **transport**: `Framed<S: Transport>`—wraps a stream for frame-based I/O.
* **message**: `Message` enum for text/binary payloads.
* **error**: `StreamError` and `FrameError` definitions.
