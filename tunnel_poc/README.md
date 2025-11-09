# Tunnel Proof-of-Concept

This crate contains a minimal Rust proof-of-concept (PoC) that forwards IP packets between two sites using a TUN interface and a UDP socket. It is a stepping stone toward the full product vision: an easy-to-use secure overlay that links private networks across the public internet.

## Highlights

- Creates a point-to-point TUN interface (layer 3) on Linux.
- Moves packets from the virtual interface to a UDP socket connected to a remote peer and vice versa.
- Supports optional keepalives to keep NAT bindings open.
- Uses async I/O (`tokio`) and structured logging (`tracing`) for observability.

> **Important:** This PoC does not yet encrypt/authenticate packets. Use only in a controlled lab.

## Building

```bash
cd tunnel_poc
cargo build
```

## Runtime Requirements

- Linux host with `CAP_NET_ADMIN` (typically run with `sudo`).
- UDP connectivity between the peers (port forwarding or firewall rules as needed).
- Matching configuration on both sides (`bind`, `peer`, tunnel addresses).

## Configuration

Copy `config.sample.toml` to `config.toml` and adjust values:

```bash
cp config.sample.toml config.toml
```

Key fields:

- `tun.address` / `tun.destination`: IPs used on the virtual link (point-to-point).
- `tun.netmask`: netmask applied locally (use `/30` for point-to-point pairs).
- `tunnel.bind`: local UDP listen address (typically `0.0.0.0:PORT`).
- `tunnel.peer`: public IP:port of the remote endpoint.
- `tunnel.keepalive_interval_secs`: optional NAT keepalive timer.

## Running

```bash
sudo cargo run -- --config config.toml
```

The program logs basic state transitions and will exit cleanly on `Ctrl+C`.

### Routing

Once the TUN interface is up you may add routes to reach the remote private subnet, for example:

```bash
sudo ip route add 10.20.0.0/24 dev poc0
```

Repeat symmetrically on the peer side so that return traffic follows the tunnel.

## Known Gaps & Next Steps

- No encryption or authentication (targets: Noise / WireGuard-style handshake).
- No automatic peer discovery or key exchange.
- Metrics and control APIs are placeholders (`stats_task`).
- Packaging is Linux-only; Android/Windows/macOS shims still to be planned.

These items map directly to upcoming milestones as we evolve from PoC to user-facing product.
