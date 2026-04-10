#!/usr/bin/env python3
"""
测试 mock.py 的 HTTP API（不需要 GDB）。
"""
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import json
import threading
import time
from http.client import HTTPConnection

# 只测试 API 层，mock GDB
from src.mock import MockAPI, BreakpointEvent


class FakeGDB:
    """模拟 GDB 会话"""
    def __init__(self):
        self._regs = {'rax': 0x1234, 'rbx': 0x5678, 'rip': 0x1000}
        self._mem = {}

    def read_register(self, name):
        return self._regs.get(name, 0)

    def read_registers(self, names):
        return {n: self._regs.get(n, 0) for n in names}

    def write_register(self, name, value):
        self._regs[name] = value

    def read_memory(self, addr, size):
        return b'\xDE\xAD\xBE\xEF'[:size]

    def set_breakpoint(self, location):
        return 42

    def delete_breakpoint(self, bp_id):
        pass

    def continue_execution(self):
        pass


def test_api():
    print("TEST: MockAPI HTTP 接口")

    fake = FakeGDB()
    api = MockAPI(fake, port=19877)
    api.start()
    time.sleep(0.3)

    conn = HTTPConnection("127.0.0.1", 19877)

    # GET /state
    conn.request("GET", "/state")
    resp = conn.getresponse()
    data = json.loads(resp.read())
    assert data["state"] == "init"
    print("  ✓ GET /state")

    # GET /registers
    conn.request("GET", "/registers")
    resp = conn.getresponse()
    data = json.loads(resp.read())
    assert data["rax"] == "0x1234"
    print(f"  ✓ GET /registers: {data}")

    # GET /memory
    conn.request("GET", "/memory?addr=0x1000&size=4")
    resp = conn.getresponse()
    data = json.loads(resp.read())
    assert data["data"] == "deadbeef"
    print(f"  ✓ GET /memory: {data['data']}")

    # POST /respond
    body = json.dumps({"value": "0xCAFE", "register": "rax"})
    conn.request("POST", "/respond", body=body, headers={"Content-Type": "application/json"})
    resp = conn.getresponse()
    assert resp.status == 200
    assert fake._regs['rax'] == 0xCAFE
    print(f"  ✓ POST /respond: rax=0x{fake._regs['rax']:x}")

    # POST /breakpoint
    body = json.dumps({"location": "*0xDEAD"})
    conn.request("POST", "/breakpoint", body=body, headers={"Content-Type": "application/json"})
    resp = conn.getresponse()
    data = json.loads(resp.read())
    assert data["id"] == 42
    print(f"  ✓ POST /breakpoint: id={data['id']}")

    # Push event
    event = BreakpointEvent(
        event_id="test-1",
        address=0xDEAD,
        bp_id=42,
        registers={"rax": 0x1111, "rip": 0xDEAD},
    )
    api.push_event(event)

    # GET /event (should get the pushed event)
    conn.request("GET", "/event")
    resp = conn.getresponse()
    data = json.loads(resp.read())
    assert data["address_hex"] == "0xdead"
    print(f"  ✓ GET /event: {data['address_hex']}")

    api.stop()
    print("\n✅ 所有 API 测试通过")


if __name__ == "__main__":
    test_api()
