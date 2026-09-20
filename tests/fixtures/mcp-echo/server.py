#!/usr/bin/env python3
"""Minimal MCP stdio echo server (real MCP protocol via the official SDK).

Used by the Hermes MCP integration tests: connect, list tools, execute the
echo tool, disconnect. Requires the ``mcp`` package (test dependency only).
"""

try:
    from mcp.server.mcpserver import MCPServer
except ImportError:  # pragma: no cover - guarded at test collection
    MCPServer = None


def build_server():
    server = MCPServer("echo")

    @server.tool()
    def echo(message: str) -> str:
        """Echo back test message."""
        return f"echo: {message}"

    return server


def main() -> None:
    build_server().run()


if __name__ == "__main__":
    main()
