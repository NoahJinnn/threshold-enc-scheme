# Threshold Encryption scheme

This document is an instruction for:

- Distributed Key Generation (DKG) workflow
- Threshold Encryption and Decryption (TED) workflow

This sample scheme involves 2 nodes `client` and `server`, both is written in Rust and implemented as HTTP server using `axum`.

Restful API is the main protocol we used here for the communication between 2 nodes in the demo.

## Prerequisite

- Install Rust toolchains. Follow is mine

```sh
rustup 1.25.2 (17db695f1 2023-02-01)
rustc 1.69.0 (84c898d65 2023-04-16)
cargo 1.69.0 (6e9a83356 2023-04-12)
```

## DKG

This scheme has 3 routes:

1 /init: 
- req: 
  - server receive client publickey `p1_pk`
- server exec:
  - generate keypair
  - create `sync_key_gen_0` instance
  - create server part `p0_part`
- resp
  - send back server publickey `p0_pk` + server part `p0_part` to client so that client can create its `sync_key_gen`

=> `server`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_0`, `p0_part`

=> `client`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_1`, `p0_part`, `p1_part`

2 /commit: 
- req:
  - server receive from client: `p1_part`, `p1_pk` and `p1_acks` list (client will run "acknowledge process" with its part `p1_part` + server part `p0_part` to validate parts and generate its `p1_acks` list)
- server exec:
    - run "acknowledge process" with its part `p0_part` + client part `p1_part` to validate parts and generate its `p0_acks` list as a result.
- resp
  - send its `p0_acks` list to client. Now both parties has all `ack` result of communication

=> `server`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_0`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`

=> `client`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_1`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`

3 /finalize:
- req:
  - client able to use its `sync_key_gen_1` instance to generate `pubkey_set` -> create a `secret_key_share` to sign message -> create `signature_share_1` => finally send `signature_share_1` + `signed message` to server
- server exec:
  - use `sync_key_gen_0` instance to generate `pubkey_set` -> create `secret_key_share` `sks0`
  - sign `signed message` that received from client with `sks0` -> create `signature_share_0`
  - create combined signature with both `signature_share_0` + `signature_share_1` 
  - verify the combined signature with the `signed message`
- resp:
  -  send back the status of verifying

=> `server`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_0`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`, `signed message`, `pks0`, `sks0`, `signature_share_0`, `signature_share_1`

=> `client`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_1`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`, `signed message`, `pks1`, `sks1`,  `signature_share_1`

Usage:

Clone this repository

```sh
cd server
cargo run # Server currently running on port 3000
```

```sh
cd client
cargo run # Server currently running on port 3001
```

Call 3 route sequencely:

```sh
curl --location --request POST 'localhost:3001/init_dkg'
curl --location --request POST 'localhost:3001/commit'
curl --location --request POST 'localhost:3001/finalize_dkg'
```

```sh
# client
2023-06-26T10:13:19.961337Z DEBUG client: listening on 127.0.0.1:3001
2023-06-26T10:15:45.821868Z DEBUG hyper::proto::h1::io: parsed 8 headers
2023-06-26T10:15:45.821981Z DEBUG hyper::proto::h1::conn: incoming body is empty
2023-06-26T10:15:45.822401Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2023-06-26T10:15:45.843770Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: reqwest::connect: starting new connection: http://127.0.0.1:3000/    
2023-06-26T10:15:45.843885Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: hyper::client::connect::http: connecting to 127.0.0.1:3000
2023-06-26T10:15:45.844302Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: hyper::client::connect::http: connected to 127.0.0.1:3000
2023-06-26T10:15:45.844617Z DEBUG hyper::proto::h1::io: flushed 301 bytes
2023-06-26T10:15:45.982462Z DEBUG hyper::proto::h1::io: parsed 3 headers
2023-06-26T10:15:45.982488Z DEBUG hyper::proto::h1::conn: incoming body is content-length (1743 bytes)
2023-06-26T10:15:45.982541Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-26T10:15:45.982680Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: hyper::client::pool: pooling idle connection for ("http", 127.0.0.1:3000)
2023-06-26T10:15:46.102605Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=280 ms status=200
2023-06-26T10:15:46.102782Z DEBUG hyper::proto::h1::io: flushed 111 bytes
2023-06-26T10:15:48.250013Z DEBUG hyper::proto::h1::io: parsed 8 headers
2023-06-26T10:15:48.250088Z DEBUG hyper::proto::h1::conn: incoming body is empty
2023-06-26T10:15:48.250372Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_request: started processing request
Node #1 handles Part from node success #0
Node #1 handles Part from node success #1
2023-06-26T10:15:48.780459Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: reqwest::connect: starting new connection: http://127.0.0.1:3000/    
2023-06-26T10:15:48.780619Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: hyper::client::connect::http: connecting to 127.0.0.1:3000
2023-06-26T10:15:48.781718Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: hyper::client::connect::http: connected to 127.0.0.1:3000
2023-06-26T10:15:48.782096Z DEBUG hyper::proto::h1::io: flushed 4279 bytes
2023-06-26T10:15:49.286709Z DEBUG hyper::proto::h1::io: parsed 3 headers
2023-06-26T10:15:49.286747Z DEBUG hyper::proto::h1::conn: incoming body is content-length (5205 bytes)
2023-06-26T10:15:49.286805Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-26T10:15:49.286968Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: hyper::client::pool: pooling idle connection for ("http", 127.0.0.1:3000)
2023-06-26T10:15:49.287385Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=1037 ms status=200
2023-06-26T10:15:49.287491Z DEBUG hyper::proto::h1::io: flushed 111 bytes
2023-06-26T10:15:51.321739Z DEBUG hyper::proto::h1::io: parsed 8 headers
2023-06-26T10:15:51.321816Z DEBUG hyper::proto::h1::conn: incoming body is empty
2023-06-26T10:15:51.322247Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2023-06-26T10:15:51.682539Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: reqwest::connect: starting new connection: http://127.0.0.1:3000/    
2023-06-26T10:15:51.682621Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::connect::http: connecting to 127.0.0.1:3000
2023-06-26T10:15:51.683126Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::connect::http: connected to 127.0.0.1:3000
2023-06-26T10:15:51.683317Z DEBUG hyper::proto::h1::io: flushed 509 bytes
2023-06-26T10:15:52.145405Z DEBUG hyper::proto::h1::io: parsed 3 headers
2023-06-26T10:15:52.145426Z DEBUG hyper::proto::h1::conn: incoming body is content-length (19 bytes)
2023-06-26T10:15:52.145454Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-26T10:15:52.145572Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::pool: pooling idle connection for ("http", 127.0.0.1:3000)
is_success: FinalizeResp { is_success: true }
2023-06-26T10:15:52.145721Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=823 ms status=200
2023-06-26T10:15:52.145821Z DEBUG hyper::proto::h1::io: flushed 111 bytes
```

```sh
# server
2023-06-26T10:13:18.714595Z DEBUG server: listening on 127.0.0.1:3000
2023-06-26T10:15:45.844910Z DEBUG hyper::proto::h1::io: parsed 4 headers
2023-06-26T10:15:45.844941Z DEBUG hyper::proto::h1::conn: incoming body is content-length (186 bytes)
2023-06-26T10:15:45.844987Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-26T10:15:45.845092Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
req_body InitDkgReq { p1_pk: PublicKey(12e3..9920) }
2023-06-26T10:15:45.982024Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=136 ms status=200
2023-06-26T10:15:45.982235Z DEBUG hyper::proto::h1::io: flushed 1853 bytes
2023-06-26T10:15:45.991346Z DEBUG hyper::proto::h1::conn: read eof
2023-06-26T10:15:48.782317Z DEBUG hyper::proto::h1::io: parsed 4 headers
2023-06-26T10:15:48.782373Z DEBUG hyper::proto::h1::conn: incoming body is content-length (4165 bytes)
2023-06-26T10:15:48.782475Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-26T10:15:48.782581Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_request: started processing request
req_body CommitReq { p1_part: Part("<degree 0>", "<2 rows>"), p1_acks: [Ack(0, "<2 values>"), Ack(1, "<2 values>")] }
2023-06-26T10:15:49.286200Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=503 ms status=200
2023-06-26T10:15:49.286397Z DEBUG hyper::proto::h1::io: flushed 5315 bytes
2023-06-26T10:15:49.287485Z DEBUG hyper::proto::h1::conn: read eof
2023-06-26T10:15:51.683476Z DEBUG hyper::proto::h1::io: parsed 4 headers
2023-06-26T10:15:51.683499Z DEBUG hyper::proto::h1::conn: incoming body is content-length (390 bytes)
2023-06-26T10:15:51.683636Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-26T10:15:51.683749Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
req_body FinalizeReq { sig_share_1: SignatureShare(0dc3..d434), signed_msg_1: "Sign this" }
is_success true
2023-06-26T10:15:52.145089Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=461 ms status=200
2023-06-26T10:15:52.145217Z DEBUG hyper::proto::h1::io: flushed 127 bytes
2023-06-26T10:15:52.145892Z DEBUG hyper::proto::h1::conn: read eof
```
