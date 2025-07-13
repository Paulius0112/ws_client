
# Protocol

- Protocol has two parts:
- Handshake

Handshake from the client looks like:
```
GET /chat HTTP/1.1
Host: server.example.com
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Origin: http://example.com
Sec-WebSocket-Protocol: chat, superchat
Sec-WebSocket-Version: 13
```

Handshake from the server looks like:
```
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
Sec-WebSocket-Protocol: chat
```

- Once the client and server have both sent their handshakes, and if the handshake was successful, then the data transfer part starts.
- Data Transfer


# Masking
- Client must send masked frames. Server MUST NOT send masked frames
0                   1                   2                   3
      0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
     +-+-+-+-+-------+-+-------------+-------------------------------+
     |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
     |I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
     |N|V|V|V|       |S|             |   (if payload len==126/127)   |
     | |1|2|3|       |K|             |                               |
     +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
     |     Extended payload length continued, if payload len == 127  |
     + - - - - - - - - - - - - - - - +-------------------------------+
     |                               |Masking-key, if MASK set to 1  |
     +-------------------------------+-------------------------------+
     | Masking-key (continued)       |          Payload Data         |
     +-------------------------------- - - - - - - - - - - - - - - - +
     :                     Payload Data continued ...                :
     + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
     |                     Payload Data continued ...                |
     +---------------------------------------------------------------+

- FIN - 1 bit. 
- RSV1 - 3. Must be 0 
- Opcode - 4 bits. interpretation of the payload data.
```
*  %x0 denotes a continuation frame
*  %x1 denotes a text frame
*  %x2 denotes a binary frame
*  %x3-7 are reserved for further non-control frames
*  %x8 denotes a connection close
*  %x9 denotes a ping
*  %xA denotes a pong
*  %xB-F are reserved for further control frames
```
- Mask - 1bit. Defines if payload data is masked
- Payload lenght - 7 bits, 7+16 bits or 7+64 bits
- Masking key - 0 or 4 bytes. All frames from client to server are masked by a 32-bit value that is contained within the frame. Mask bit must be set to 1
- Payload data
- Extension data - must be set to 0 bytes unless an extension has been negotiated.
- Application data - taking up the remainder ofn the frame after any extension data. Its lenght is equal to the payload lenght minus the lenght of the extension data


# XOR
- Takes two bites and returns 1 if they differ, 0 if they're the same


# Client validation
 1.  If the status code received from the server is not 101, the
       client handles the response per HTTP [RFC2616] procedures.  In
       particular, the client might perform authentication if it
       receives a 401 status code; the server might redirect the client
       using a 3xx status code (but clients are not required to follow
       them), etc.  Otherwise, proceed as follows.

   2.  If the response lacks an |Upgrade| header field or the |Upgrade|
       header field contains a value that is not an ASCII case-
       insensitive match for the value "websocket", the client MUST
       _Fail the WebSocket Connection_.

   3.  If the response lacks a |Connection| header field or the
       |Connection| header field doesn't contain a token that is an
       ASCII case-insensitive match for the value "Upgrade", the client
       MUST _Fail the WebSocket Connection_.

   4.  If the response lacks a |Sec-WebSocket-Accept| header field or
       the |Sec-WebSocket-Accept| contains a value other than the
       base64-encoded SHA-1 of the concatenation of the |Sec-WebSocket-
       Key| (as a string, not base64-decoded) with the string "258EAFA5-
       E914-47DA-95CA-C5AB0DC85B11" but ignoring any leading and
       trailing whitespace, the client MUST _Fail the WebSocket
       Connection_.

   5.  If the response includes a |Sec-WebSocket-Extensions| header
       field and this header field indicates the use of an extension
       that was not present in the client's handshake (the server has
       indicated an extension not requested by the client), the client
       MUST _Fail the WebSocket Connection_.  (The parsing of this
       header field to determine which extensions are requested is
       discussed in Section 9.1.)

   6.  If the response includes a |Sec-WebSocket-Protocol| header field
       and this header field indicates the use of a subprotocol that was
       not present in the client's handshake (the server has indicated a
       subprotocol not requested by the client), the client MUST _Fail
       the WebSocket Connection_.