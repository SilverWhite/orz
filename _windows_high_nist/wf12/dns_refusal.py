"""TER T2.2 (W-F12) 本地 DNS 拒答器（可部署组件，2026-09-04）。

在 guest 墙内以 loopback DNS（127.0.0.1:53）提供「非 allowlist 域名立即
NXDOMAIN」的快速确定性失败——替代黑洞式等待超时。allowlist 语义：

- 默认不携带域名 allowlist：所有 A/AAAA 查询一律 NXDOMAIN（评测允许端
  以 `--allowlist-ip` 直连 IP 使用，DNS 面全局拒绝，不泄露 allowlist 域
  内容）；
- 显式 `--allowlist-domains a,b` + `--upstream-dns ip` 时，仅 allowlist
  域名经上游转发（测试/调试面；无上游配置时 allowlist 携带 = 启动失败
  fail-closed）。

用法（VM 墙内，需管理员设置适配器 DNS）：
  python dns_refusal.py --port 53 [--allowlist-domains ...] [--upstream-dns ...]

TCP 快速拒答层为独立组件（WFP/本地透明层实测后接入）；本模块只负责 DNS
面。仅标准库。
"""

import argparse
import socket
import struct
import threading


def _parse_question(packet: bytes):
    """从 DNS 查询报文解出（question 原样字节, qname 小写域名）。"""
    if len(packet) < 12:
        raise ValueError("packet too short")
    idx = 12
    labels = []
    while idx < len(packet):
        length = packet[idx]
        if length == 0:
            idx += 1
            break
        if length & 0xC0 == 0xC0:
            # 压缩指针不应出现在查询 qname，遇到即拒绝。
            raise ValueError("compressed pointer in query qname")
        idx += 1
        if idx + length > len(packet):
            raise ValueError("qname overruns packet")
        labels.append(packet[idx : idx + length])
        idx += length
    qname_end = idx
    qname = ".".join(part.decode("ascii", "replace").lower() for part in labels)
    # qtype/qclass（2+2 字节）
    if idx + 4 > len(packet):
        raise ValueError("missing qtype/qclass")
    return packet[12 : qname_end + 4], qname


def nxdomain_response(query: bytes) -> bytes:
    """把查询改写成 NXDOMAIN 应答（rcode=3），保留 ID/QR/QD。"""
    if len(query) < 12:
        raise ValueError("query too short")
    header = bytearray(query[:12])
    flags = struct.unpack(">H", header[2:4])[0]
    flags = (flags & 0xFE00) | 0x8183  # QR=1, RD 保留, rcode=NXDOMAIN
    header[2:4] = struct.pack(">H", flags)
    header[4:6] = struct.pack(">H", 1)  # QDCOUNT=1
    header[6:12] = b"\x00" * 6  # AN/NS/AR = 0
    return bytes(header) + query[12:]


class DnsRefusalServer:
    """UDP loopback DNS 拒答器（threading；供 guest 与测试复用）。"""

    def __init__(self, port=0, allowlist_domains=(), upstream_dns=None):
        self._allowlist = {d.lower() for d in allowlist_domains}
        if self._allowlist and not upstream_dns:
            raise ValueError("allowlist-domains requires --upstream-dns (fail-closed)")
        self._upstream = upstream_dns
        self._sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self._sock.bind(("127.0.0.1", port))
        self._port = self._sock.getsockname()[1]
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._serve, daemon=True)

    @property
    def port(self) -> int:
        return self._port

    def start(self):
        self._thread.start()

    def stop(self):
        self._stop.set()
        # 向自身发一包以唤醒 recvfrom。
        try:
            self._sock.sendto(b"\x00" * 12, ("127.0.0.1", self._port))
        except OSError:
            pass
        self._thread.join(timeout=1.0)

    def _forward(self, packet: bytes) -> bytes:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        try:
            s.settimeout(2.0)
            s.sendto(packet, (self._upstream, 53))
            data, _ = s.recvfrom(4096)
            return data
        except OSError:
            # 上游失败 → SERVFAIL（可判定失败，非黑洞）。
            header = bytearray(packet[:12])
            flags = struct.unpack(">H", header[2:4])[0]
            header[2:4] = struct.pack(">H", (flags & 0xFE00) | 0x8182)
            return bytes(header) + packet[12:]
        finally:
            s.close()

    def _serve(self):
        while not self._stop.is_set():
            try:
                data, addr = self._sock.recvfrom(4096)
            except OSError:
                break
            if len(data) < 12:
                continue
            try:
                question, qname = _parse_question(data)
            except ValueError:
                continue
            del question
            if qname in self._allowlist:
                response = self._forward(data)
            else:
                response = nxdomain_response(data)
            try:
                self._sock.sendto(response, addr)
            except OSError:
                pass


def _selftest() -> None:
    """本地自测：127.0.0.1 临时端口发 A 查询 → NXDOMAIN 且 rcode=3。"""
    server = DnsRefusalServer()
    server.start()
    try:
        client = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        client.settimeout(2.0)
        # 手工构造 minimal A 查询（name=example.com, type=A, class=IN）。
        qname = b"\x07example\x03com\x00"
        query = struct.pack(">HHHHHH", 0x1234, 0x0100, 1, 0, 0, 0) + qname + struct.pack(">HH", 1, 1)
        client.sendto(query, ("127.0.0.1", server.port))
        response, _ = client.recvfrom(4096)
        flags = struct.unpack(">H", response[2:4])[0]
        assert flags & 0x000F == 3, f"expected NXDOMAIN rcode, flags={flags:#x}"
        assert struct.unpack(">H", response[4:6])[0] == 1, "QDCOUNT must be preserved"
        print("DNS_REFUSAL_SELFTEST_OK")
    finally:
        server.stop()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=53)
    parser.add_argument("--allowlist-domains", default="")
    parser.add_argument("--upstream-dns", default="")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        _selftest()
        return 0
    # TER 全面审查 M2W-3 (2026-09-04)：空段（",," / 逗号尾随）是配置错误，
    # 静默忽略会让人以为域名已放行——fail-closed 显式报错。
    raw_segments = args.allowlist_domains.split(",") if args.allowlist_domains else []
    empty_hits = [seg for seg in raw_segments if not seg.strip()]
    if empty_hits:
        print(
            "DNS_REFUSAL_FAILED allowlist-domains contains empty segments "
            f"(parsed={raw_segments!r}) — refusing to start",
            flush=True,
        )
        return 2
    allowlist = [seg.strip().lower() for seg in raw_segments]
    try:
        server = DnsRefusalServer(
            port=args.port,
            allowlist_domains=allowlist,
            upstream_dns=args.upstream_dns or None,
        )
    except (OSError, ValueError) as exc:
        # TER 全面审查 M2W-3：bind/权限/端口占用在启动前 fail-fast，给可读
        # 错误而不是让线程悄悄死掉或等运行期才暴露。
        print(f"DNS_REFUSAL_FAILED startup preflight error: {exc}", flush=True)
        return 2
    print(f"DNS_REFUSAL_LISTENING 127.0.0.1:{server.port} allowlist={allowlist}")
    server.start()
    try:
        while True:
            threading.Event().wait(3600)
    except KeyboardInterrupt:
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
