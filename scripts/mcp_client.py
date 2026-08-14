#!/usr/bin/env python3
"""
MCP 客户端 — 通过 stdio 与 geo-toolbox MCP server 通信
"""
import subprocess, json, sys, time, os

class MCPClient:
    def __init__(self, binary):
        self.proc = subprocess.Popen(
            [binary, 'mcp-serve', '--port', '9378'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, bufsize=1
        )
        self._id = 0
        self._initialized = False

    def _send(self, method, params=None):
        self._id += 1
        req = {'jsonrpc': '2.0', 'id': self._id, 'method': method}
        if params:
            req['params'] = params
        self.proc.stdin.write(json.dumps(req, ensure_ascii=False) + '\n')
        self.proc.stdin.flush()

    def _recv(self, timeout=10):
        import select
        start = time.time()
        buf = ''
        while time.time() - start < timeout:
            line = self.proc.stdout.readline()
            if not line:
                time.sleep(0.1)
                continue
            line = line.strip()
            if line.startswith('{'):
                try:
                    return json.loads(line)
                except:
                    pass
        raise TimeoutError("No response")

    def initialize(self):
        self._send('initialize', {
            'protocolVersion': '2024-11-05',
            'capabilities': {},
            'clientInfo': {'name': 'cli', 'version': '1.0'}
        })
        resp = self._recv()
        self._initialized = True
        return resp

    def call(self, method, params=None):
        if not self._initialized:
            self.initialize()
        self._send(method, params)
        return self._recv()

    def close(self):
        self.proc.terminate()

client = MCPClient('D:/geo/geo-toolbox/target/debug/geo-toolbox.exe')

# List tools
r = client.call('tools/list')
tools = r.get('result', {}).get('tools', [])
print(f"Tools ({len(tools)}):")
for t in tools:
    print(f"  {t['name']}: {t.get('description','')[:100]}")

# Try stac_search
r2 = client.call('tools/call', {
    'name': 'stac_search',
    'arguments': {
        'collection': 'modis-13Q1-061',
        'min_lon': 110, 'min_lat': 35,
        'max_lon': 115, 'max_lat': 40,
        'date_from': '2015-08-01', 'date_to': '2015-08-31',
        'limit': 3
    }
})
print(f"\nSTAC result: {json.dumps(r2, ensure_ascii=False, indent=2)[:500]}")

client.close()
