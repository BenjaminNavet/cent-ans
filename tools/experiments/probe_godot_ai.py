"""Probe godot-ai over MCP stdio and measure the context cost of its tools.

Usage: uv run --with mcp python probe_godot_ai.py <step> [json-args]
  step = tools            -> size of the tool schemas (context cost of just enabling the server)
  step = call NAME ARGS   -> call one tool, report text size and image dimensions
"""

import asyncio
import base64
import json
import os
import struct
import sys

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

ATTACH_COMMAND = [
    "--isolated",
    "--no-config",
    "--from",
    "godot-ai==4.2.3",
    "godot-ai",
    "attach",
    "--port",
    "8000",
    "--ws-port",
    "9500",
    "--disable-telemetry",
    *os.environ.get("GAI_EXCLUDE", "").split(),
]


def png_size(raw_bytes):
    """Return (width, height) of a PNG, or None."""
    if raw_bytes[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", raw_bytes[16:24])


def jpeg_size(raw_bytes):
    """Return (width, height) of a JPEG, or None."""
    index = 2
    while index < len(raw_bytes):
        if raw_bytes[index] != 0xFF:
            return None
        marker = raw_bytes[index + 1]
        length = struct.unpack(">H", raw_bytes[index + 2 : index + 4])[0]
        if marker in (0xC0, 0xC2):
            height, width = struct.unpack(">HH", raw_bytes[index + 5 : index + 9])
            return width, height
        index += 2 + length
    return None


async def main():
    """Connect to godot-ai and run the requested probe step."""
    server = StdioServerParameters(
        command="uvx",
        args=ATTACH_COMMAND,
        env={
            **os.environ,
            "GODOT_AI_DISABLE_TELEMETRY": "true",
            "UV_HTTP_TIMEOUT": "300",
        },
    )
    async with (
        stdio_client(server) as (read_stream, write_stream),
        ClientSession(read_stream, write_stream) as session,
    ):
        await session.initialize()
        step = sys.argv[1]
        if step == "tools":
            listing = await session.list_tools()
            schema_json = json.dumps(
                [tool.model_dump(exclude_none=True) for tool in listing.tools]
            )
            print(
                f"tools={len(listing.tools)} schema_chars={len(schema_json)} approx_tokens={len(schema_json) // 4}"
            )
            for tool in listing.tools:
                print(
                    f"  {tool.name}: {len(json.dumps(tool.model_dump(exclude_none=True))) // 4} tok"
                )
            return
        if step == "schema":
            listing = await session.list_tools()
            for tool in listing.tools:
                if tool.name in sys.argv[2:]:
                    print(tool.name, "::", tool.description[:1800])
                    print(
                        json.dumps(
                            tool.input_schema.get("properties", {}), ensure_ascii=False
                        )[:2500]
                    )
            return
        tool_name = sys.argv[2]
        arguments = json.loads(sys.argv[3]) if len(sys.argv) > 3 else {}
        result = await session.call_tool(tool_name, arguments)
        for block in result.content:
            if block.type == "text":
                print(
                    f"text chars={len(block.text)} approx_tokens={len(block.text) // 4}"
                )
                print(block.text[:1500])
                if len(sys.argv) > 4:
                    with open(sys.argv[4] + ".txt", "w") as text_file:
                        text_file.write(block.text)
            elif block.type == "image":
                raw_bytes = base64.b64decode(block.data)
                dimensions = png_size(raw_bytes) or jpeg_size(raw_bytes)
                out_path = sys.argv[4] if len(sys.argv) > 4 else "shot.img"
                with open(out_path, "wb") as image_file:
                    image_file.write(raw_bytes)
                tokens = dimensions[0] * dimensions[1] // 750 if dimensions else "?"
                print(
                    f"image {block.mime_type} {dimensions} bytes={len(raw_bytes)} approx_tokens={tokens} saved={out_path}"
                )


asyncio.run(main())
