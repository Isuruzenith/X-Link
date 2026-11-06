#!/usr/bin/env python3
"""Generate exact-scale ASCII wireframes for X-Link.

Scale: 1 column = 10 px, 1 row = 20 px.
Expanded reference window : 1000 x 680 px -> 100 cols x 34 rows
Minimum window            :  720 x 520 px ->  72 cols x 26 rows
"""


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.g = [[" "] * w for _ in range(h)]
        self.boxes = []

    def put(self, x, y, s):
        for i, ch in enumerate(s):
            xx = x + i
            if 0 <= xx < self.w and 0 <= y < self.h:
                self.g[y][xx] = ch
            else:
                raise ValueError(f"text out of canvas at ({xx},{y}): {s!r}")

    def box(self, x, y, w, h, title=None, right=None):
        if x + w > self.w or y + h > self.h:
            raise ValueError(f"box out of canvas {x},{y},{w},{h}")
        for i in range(w):
            self.g[y][x + i] = "-"
            self.g[y + h - 1][x + i] = "-"
        for j in range(h):
            self.g[y + j][x] = "|"
            self.g[y + j][x + w - 1] = "|"
        for cx, cy in ((x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)):
            self.g[cy][cx] = "+"
        self.boxes.append((x, y, w, h))
        if title:
            self.put(x + 2, y, f" {title} ")
        if right:
            self.put(x + w - len(right) - 3, y, f" {right} ")

    def clear(self, x, y, w, h):
        for j in range(h):
            for i in range(w):
                self.g[y + j][x + i] = " "

    def vline(self, x, y0, y1):
        for j in range(y0, y1 + 1):
            self.g[j][x] = "|"

    def hline(self, x0, x1, y, ch="-"):
        for i in range(x0, x1 + 1):
            self.g[y][i] = ch

    def check(self):
        bad = []
        for (x, y, w, h) in self.boxes:
            for j in range(1, h - 1):
                for cx in (x, x + w - 1):
                    if self.g[y + j][cx] not in "|+":
                        bad.append((cx, y + j))
        return bad

    def render(self):
        bad = self.check()
        if bad:
            raise ValueError(f"border overwritten at {bad[:6]}")
        return "\n".join("".join(r).rstrip() for r in self.g)


# --------------------------------------------------------------------------
# Shell
# --------------------------------------------------------------------------
NAV = [
    ("Dashboard", "spd"),
    ("Profiles", "fld"),
    ("Routing", "rte"),
    ("Connections", "hub"),
    ("Logs", "trm"),
    ("Settings", "set"),
]


def shell(w, h, rail, active, title, subtitle, status="(o) Protected 01:24:10"):
    """rail = 22 (expanded, 220px) or 8 (medium, 80px)."""
    c = Canvas(w, h)
    c.box(0, 0, w, h)
    c.vline(rail, 1, h - 2)
    c.g[0][rail] = "+"
    c.g[h - 1][rail] = "+"
    if rail >= 20:
        c.put(2, 1, "[X] X-Link")
        c.put(2, 2, "Network control")
    else:
        c.put(2, 1, "[X]")
    # nav items
    y = 4
    for i, (label, icon) in enumerate(NAV):
        if rail >= 20:
            txt = f"[{icon}] {label}"
            c.put(3, y, txt)
            if i == active:
                c.put(1, y, ">")
        else:
            c.put(2, y, "[*]" if i == active else "[ ]")
        y += 2
    # session pill at bottom
    if rail >= 20:
        c.put(2, h - 3, "(o) Connected")
        c.put(2, h - 2, "    Proxy session")
    else:
        c.put(3, h - 2, "(o)")
    # header
    cx = rail + 2
    c.put(cx, 1, title)
    c.put(cx, 2, subtitle)
    c.put(w - len(status) - 3, 1, status)
    c.hline(rail + 1, w - 2, 3)
    return c


def body_box(rail, w, h):
    """Return (x, y, bw, bh) of the scroll-less content area inside shell."""
    x = rail + 2
    y = 4
    return x, y, w - x - 2, h - y - 1


# --------------------------------------------------------------------------
# Views, expanded (100 x 34)
# --------------------------------------------------------------------------
def dashboard_expanded():
    W, H, R = 100, 34, 22
    c = shell(W, H, R, 0, "Dashboard", "Your connection at a glance")
    x, y, bw, bh = body_box(R, W, H)  # x=24, y=4, bw=74, bh=29
    # hero
    c.box(x, y, 28, 6, "Connect", "TUN routing")
    c.put(x + 2, y + 1, "Connected")
    c.put(x + 2, y + 2, "via US-West-Reality")
    c.put(x + 2, y + 3, "[ Disconnect          ]")
    c.put(x + 2, y + 4, "Session active 01:24:10")
    tx = x + 29
    c.box(tx, y, bw - 29, 6, "Live telemetry", "Live")
    tiles = [("Download", "14.2 MB/s"), ("Upload", "1.8 MB/s"),
             ("Active", "42"), ("Core", "38 MB")]
    tw = (bw - 29 - 3) // 4
    for i, (k, v) in enumerate(tiles):
        c.put(tx + 2 + i * tw, y + 2, k)
        c.put(tx + 2 + i * tw, y + 3, v)
    # main grid
    gy = y + 7  # 11
    gh = bh - 7  # 22
    lw = 38
    c.box(x, gy, lw, gh, "Servers (12)", "[Ping all]")
    c.put(x + 2, gy + 1, "[ Search servers...          ]")
    c.put(x + 2, gy + 2, "Server            Proto   Ping")
    rows = [("* US-West-Reality", "VLESS", "42ms"), ("  SG-Direct-Hy2", "HY2", "58ms"),
            ("  JP-Tokyo-Trojan", "TROJAN", "91ms"), ("  Frankfurt-WG", "WG", "143ms"),
            ("  UK-London-VMess", "VMESS", "188ms"), ("  CA-Toronto-SS", "SS", "Err")]
    for i, (n, p, ms) in enumerate(rows):
        c.put(x + 2, gy + 4 + i * 2, f"{n:<18}{p:<8}{ms}")
    c.put(x + 2, gy + gh - 3, "  ... internal scroll only ...")
    c.put(x + 2, gy + gh - 2, "(list scrolls, page does not)")
    rx = x + lw + 1
    rw = bw - lw - 1
    c.box(rx, gy, rw, 12, "Live traffic", "Live")
    c.put(rx + 2, gy + 1, "Down 14.2 MB/s    Up 1.8 MB/s")
    c.put(rx + 2, gy + 3, "20 MB/s |        .--.")
    c.put(rx + 2, gy + 4, "        |   .--'    '--.")
    c.put(rx + 2, gy + 5, "        | .'            '.")
    c.put(rx + 2, gy + 6, " 0 B/s  +-------------------*")
    c.put(rx + 2, gy + 9, "chart fills card")
    c.box(rx, gy + 12, rw, gh - 12, "Session")
    for i, (k, v) in enumerate([("Profile", "Premium"), ("Mode", "TUN"),
                                ("Sockets", "42"), ("Core", "1.13.14")]):
        c.put(rx + 2, gy + 13 + i, f"{k:<9}{v}")
    return c.render()


def profiles_expanded():
    W, H, R = 100, 34, 22
    c = shell(W, H, R, 1, "Profiles", "Subscriptions, files and manual servers", "[+ Add server]")
    x, y, bw, bh = body_box(R, W, H)
    lw = 32
    c.box(x, y, lw, 12, "Import profile")
    c.put(x + 2, y + 1, "[ Name (optional)         ]")
    c.put(x + 2, y + 2, "[ URL or raw config       ]")
    c.put(x + 2, y + 3, "[                         ]")
    c.put(x + 2, y + 4, "[                         ]")
    c.put(x + 2, y + 6, "Detected: 7 nodes")
    c.put(x + 2, y + 7, "VLESS 5 - TROJAN 2")
    c.put(x + 2, y + 9, "[Import] [Choose file]")
    c.box(x, y + 12, lw, bh - 12, "Saved profiles (3)")
    c.put(x + 2, y + 13, "(*) Premium Sub      ACTIVE")
    c.put(x + 2, y + 14, "    12 nodes - 2h ago")
    c.put(x + 2, y + 16, "( ) Work VPN         FILE")
    c.put(x + 2, y + 17, "    4 nodes")
    c.put(x + 2, y + 19, "( ) Manual           MANUAL")
    c.put(x + 2, y + 20, "    1 node")
    c.put(x + 2, y + 22, "-- list scrolls internally --")
    rx = x + lw + 1
    rw = bw - lw - 1
    c.box(rx, y, rw, bh, "Servers in Premium Sub", "[Ping all]")
    c.put(rx + 2, y + 1, "[ Search...   ] Auto [6h v]")
    c.put(rx + 2, y + 3, "Server           Proto  Ping  Act")
    rows = [("US-West-Reality", "VLESS", "42ms"), ("SG-Direct-Hy2", "HY2", "58ms"),
            ("JP-Tokyo-Trojan", "TROJAN", "91ms"), ("Frankfurt-WG", "WG", "143ms"),
            ("UK-London-VMess", "VMESS", "188ms")]
    for i, (n, p, ms) in enumerate(rows):
        c.put(rx + 2, y + 5 + i * 2, f"{n:<17}{p:<7}{ms:<6}[e][x]")
    c.put(rx + 2, y + bh - 4, "-- table scrolls internally --")
    c.put(rx + 2, y + bh - 2, "[Diff] [Refresh] [Delete]")
    return c.render()


def routing_expanded():
    W, H, R = 100, 34, 22
    c = shell(W, H, R, 2, "Routing", "Final outbound, rule sets and rules", "[+ Add rule]")
    x, y, bw, bh = body_box(R, W, H)
    hw = (bw - 1) // 2
    c.box(x, y, hw, 8, "Global policy")
    c.put(x + 2, y + 2, "Final outbound   [ Proxy v ]")
    c.put(x + 2, y + 4, "Bypass LAN / private    (o--)")
    c.put(x + 2, y + 6, "Applies on next connect")
    c.box(x + hw + 1, y, bw - hw - 1, 8, "Rule sets (2)", "[+ Add]")
    c.put(x + hw + 3, y + 2, "geosite-cn  remote [Update][x]")
    c.put(x + hw + 3, y + 4, "geoip-cn    remote [Update][x]")
    c.box(x, y + 9, bw, bh - 9, "Rules (priority order)")
    c.put(x + 2, y + 10, "On   #   Type            Value               Route    Act")
    rows = [("on ", "1", "domain_suffix", "example.com", "proxy"),
            ("on ", "2", "geosite", "cn", "direct"),
            ("off", "3", "process_name", "chrome.exe", "direct"),
            ("on ", "4", "ip_cidr", "10.0.0.0/8", "direct")]
    for i, (o, n, t, v, r) in enumerate(rows):
        c.put(x + 2, y + 12 + i * 2, f"{o}  {n:<3} {t:<15} {v:<19} {r:<8} [e][x]")
    c.put(x + 2, y + bh - 3, "-- rules table scrolls internally --")
    return c.render()


def connections_expanded():
    W, H, R = 100, 34, 22
    c = shell(W, H, R, 3, "Connections", "Live sockets through the tunnel", "[Close all]")
    x, y, bw, bh = body_box(R, W, H)
    c.put(x, y, "[ Search host, IP or rule... ]  (All)(Proxy)(Direct)       42 / 42 active")
    c.box(x, y + 2, bw, bh - 2, "Active sockets")
    c.put(x + 2, y + 3, "Destination         Net Rule/Chain      Speed     Total   Time  Kill")
    rows = [("api.github.com:443", "TCP", "proxy > US-West", "120 KB/s", "4.2 MB", "01:02"),
            ("1.1.1.1:53", "UDP", "direct", "0 B/s", "2 KB", "00:11"),
            ("cdn.example.net:443", "TCP", "proxy > US-West", "2.1 MB/s", "88 MB", "12:40")]
    for i, (d, n, ch, sp, tot, t) in enumerate(rows):
        c.put(x + 2, y + 5 + i * 2, f"{d:<20}{n:<4}{ch:<16}{sp:<10}{tot:<8}{t:<6}[x]")
    c.put(x + 2, y + bh - 3, "-- virtualized rows (36px), list scrolls internally --")
    return c.render()


def logs_expanded():
    W, H, R = 100, 34, 22
    c = shell(W, H, R, 4, "Logs", "Core and app events", "[Copy] [Clear]")
    x, y, bw, bh = body_box(R, W, H)
    c.put(x, y, "[ Find a message...  ] (All)(Info)(Warn)(Err)(Sys)  Auto (--o)")
    c.box(x, y + 2, bw, bh - 2, "Console", "Showing 2000 of 2000")
    lines = [("12:00:01", "INFO", "[TUN] interface up"),
             ("12:00:01", "INFO", "[DNS] fakeip pool ready"),
             ("12:00:04", "WARN", "[WARN] node slow: SG-Direct-Hy2"),
             ("12:00:09", "ERR ", "[ERR] dial tcp: i/o timeout"),
             ("12:00:10", "SYS ", "[Self-Heal] stage 2 uTLS firefox")]
    for i, (t, l, m) in enumerate(lines):
        c.put(x + 2, y + 3 + i, f"{t}  {l}  {m}")
    c.put(x + 2, y + bh - 3, "-- 28px rows, virtualized, internal scroll --")
    return c.render()


def settings_expanded():
    W, H, R = 100, 34, 22
    c = shell(W, H, R, 5, "Settings", "Mode, network and diagnostics")
    x, y, bw, bh = body_box(R, W, H)
    nw = 18
    c.box(x, y, nw, bh, "Sections")
    for i, s in enumerate(["General", "TUN Mode", "Sniffing", "DNS", "API", "Diagnostics"]):
        c.put(x + 2, y + 2 + i * 2, ("[*] " if i == 0 else "[ ] ") + s)
    cx = x + nw + 1
    cw = bw - nw - 1
    c.box(cx, y, cw, 10, "Proxy mode", "Next connect")
    c.put(cx + 2, y + 2, "(*) TUN        ( ) System proxy")
    c.put(cx + 2, y + 4, "TUN needs elevated helper on this OS")
    c.put(cx + 2, y + 6, "[Run as administrator for TUN]")
    c.box(cx, y + 10, cw, 8, "App preferences", "Live")
    c.put(cx + 2, y + 12, "Close to tray              (--o)")
    c.put(cx + 2, y + 14, "Launch on login            (o--)")
    c.box(cx, y + 18, cw, bh - 18, "Appearance", "Live")
    c.put(cx + 2, y + 20, "(Light) (Dark) (System)")
    c.put(cx + 2, y + bh - 4, "-- content pane scrolls internally --")
    return c.render()


def node_editor_sheet():
    W, H, R = 100, 34, 22
    c = shell(W, H, R, 0, "Dashboard", "Your connection at a glance")
    sw = 50
    sx = W - sw - 1
    c.clear(sx, 1, sw, H - 2)
    c.box(sx, 1, sw, H - 2, "Edit server", "[x]")
    c.put(sx + 2, 3, "(Basic)(Transport)(TLS)")
    c.put(sx + 2, 5, "Protocol          [ VLESS v ]")
    c.put(sx + 2, 7, "Name (tag)        [ US-West-Reality ]")
    c.put(sx + 2, 9, "Server            [ 198.51.100.42 ]")
    c.put(sx + 2, 11, "Port              [ 443 ]")
    c.put(sx + 2, 13, "UUID              [ 7f1c...e2 ]")
    c.put(sx + 2, 15, "Flow              [ xtls-rprx-vision v ]")
    c.put(sx + 2, H - 8, "-- body scrolls, footer pinned --")
    c.hline(sx + 1, sx + sw - 2, H - 5)
    c.put(sx + 2, H - 4, "[Cancel]                       [Save]")
    c.put(sx - 14, 12, "scrim 52%")
    return c.render()


# --------------------------------------------------------------------------
# Medium / minimum window (72 x 26), rail 8 cols
# --------------------------------------------------------------------------
def dashboard_min():
    W, H, R = 72, 26, 8
    c = shell(W, H, R, 0, "Dashboard", "At a glance", "(o) Protected")
    x, y, bw, bh = body_box(R, W, H)  # x=10 y=4 bw=60 bh=21
    c.box(x, y, 24, 5, "Connect", "TUN")
    c.put(x + 2, y + 1, "Connected")
    c.put(x + 2, y + 2, "[ Disconnect   ]")
    c.put(x + 2, y + 3, "01:24:10")
    tx = x + 25
    c.box(tx, y, bw - 25, 5, "Telemetry", "Live")
    c.put(tx + 2, y + 1, "Down 14.2 MB/s   Up 1.8 MB/s")
    c.put(tx + 2, y + 2, "Sockets 42       Core 38 MB")
    c.put(tx + 2, y + 3, "[sparkline.........]")
    gy = y + 6
    gh = bh - 6
    c.box(x, gy, bw, gh, "Servers (12)", "[Ping all]")
    c.put(x + 2, gy + 1, "[ Search...        ]")
    c.put(x + 2, gy + 3, "* US-West-Reality    VLESS   42ms")
    c.put(x + 2, gy + 5, "  SG-Direct-Hy2      HY2     58ms")
    c.put(x + 2, gy + 7, "  JP-Tokyo-Trojan    TROJAN  91ms")
    c.put(x + 2, gy + gh - 2, "-- internal scroll; chart moves to a tab --")
    return c.render()


def compact_drawer():
    W, H = 56, 22
    c = Canvas(W, H)
    c.box(0, 0, W, H)
    c.put(2, 1, "[=] Dashboard                 (o) Protected")
    c.hline(1, W - 2, 3)
    c.put(2, 5, "Single column, cards stack,")
    c.put(2, 6, "page body never scrolls sideways.")
    c.box(2, 8, 30, 12, "Modal drawer (260px)")
    c.put(4, 10, "[X] X-Link")
    c.put(4, 12, "[*] Dashboard")
    c.put(4, 13, "[ ] Profiles")
    c.put(4, 14, "[ ] Routing")
    c.put(4, 15, "[ ] Connections")
    c.put(4, 16, "[ ] Logs")
    c.put(4, 17, "[ ] Settings")
    c.put(35, 12, "scrim")
    return c.render()


ALL = {
    "dashboard_expanded": dashboard_expanded,
    "profiles_expanded": profiles_expanded,
    "routing_expanded": routing_expanded,
    "connections_expanded": connections_expanded,
    "logs_expanded": logs_expanded,
    "settings_expanded": settings_expanded,
    "node_editor_sheet": node_editor_sheet,
    "dashboard_min": dashboard_min,
    "compact_drawer": compact_drawer,
}

if __name__ == "__main__":
    import sys
    name = sys.argv[1] if len(sys.argv) > 1 else None
    for k, fn in ALL.items():
        if name and k != name:
            continue
        print("###", k)
        print(fn())
        print()
