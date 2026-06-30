"""
IPython magics for geo-toolbox — cell magic %%geo, line magics %geo, %geo_list, %geo_schema.

Lazy-loads the native Rust backend (_geo_toolbox.Geo) on first use.
"""

import json
from IPython.core.magic import Magics, magics_class, cell_magic, line_magic
from IPython.display import display, JSON


@magics_class
class GeoMagics(Magics):
    """geo-toolbox IPython magics.

    Provides:
        %%geo  — cell magic for JSON-parameterised tool calls
        %geo   — line magic for one-line tool calls
        %geo_list   — list all registered tools
        %geo_schema — show JSON schema for a tool
    """

    _geo_instance = None

    @classmethod
    def _get_geo(cls):
        """Lazy-load and cache the Geo instance from native bindings."""
        if cls._geo_instance is None:
            from geo_toolbox._geo_toolbox import Geo
            cls._geo_instance = Geo()
        return cls._geo_instance

    # ── %%geo cell magic ──────────────────────────────────────────

    @cell_magic
    def geo(self, line: str, cell: str):
        """Call a geo-toolbox tool with JSON params.

        First line of cell body: tool_name
        Remaining lines: JSON params object

        Example:
            %%geo geohash_encode
            {"lat": 39.9, "lon": 116.4, "precision": 8}
        """
        geo = self._get_geo()
        tool_name = line.strip()
        params_str = cell.strip()

        if not tool_name:
            self._print_error("Usage: %%geo <tool_name>")
            return

        # Validate JSON early
        try:
            json.loads(params_str)
        except json.JSONDecodeError as e:
            self._print_error(f"Invalid JSON params: {e}")
            return

        try:
            result = geo.call(tool_name, params_str)
            self._display_result(tool_name, result)
        except Exception as e:
            self._print_error(f"Error calling '{tool_name}': {e}")

    # ── %geo line magic ───────────────────────────────────────────

    @line_magic
    def geo_line(self, line: str):
        """One-line tool call: %geo <tool_name> <json_params>

        Example:
            %geo geohash_encode {"lat": 39.9, "lon": 116.4, "precision": 8}
        """
        parts = line.split(None, 1)
        if len(parts) < 1:
            self._print_error("Usage: %geo <tool_name> <json_params>")
            return

        tool_name = parts[0]
        params_str = parts[1] if len(parts) > 1 else "{}"

        geo = self._get_geo()

        # Validate JSON
        try:
            json.loads(params_str)
        except json.JSONDecodeError as e:
            self._print_error(f"Invalid JSON params: {e}")
            return

        try:
            result = geo.call(tool_name, params_str)
            self._display_result(tool_name, result)
        except Exception as e:
            self._print_error(f"Error calling '{tool_name}': {e}")

    # ── %geo_list magic ───────────────────────────────────────────

    @line_magic
    def geo_list(self, line: str):
        """List all registered geo-toolbox tools.

        Example:
            %geo_list
        """
        geo = self._get_geo()
        try:
            tools = json.loads(geo.list_tools())
            if not tools:
                print("No tools registered.")
                return
            print(f"\n\x1b[1m\x1b[36mgeo-toolbox registered tools ({len(tools)}):\x1b[0m\n")
            for t in tools:
                name = t.get("name", "?")
                desc = t.get("description", "")
                # truncate description to 80 chars
                if len(desc) > 80:
                    desc = desc[:77] + "..."
                print(f"  \x1b[1m{name}\x1b[0m")
                if desc:
                    print(f"    {desc}")
                print()
        except Exception as e:
            self._print_error(f"Error listing tools: {e}")

    # ── %geo_schema magic ─────────────────────────────────────────

    @line_magic
    def geo_schema(self, line: str):
        """Show JSON schema for a tool.

        Example:
            %geo_schema geohash_encode
        """
        tool_name = line.strip()
        if not tool_name:
            self._print_error("Usage: %geo_schema <tool_name>")
            return

        geo = self._get_geo()
        try:
            schema_str = geo.tool_schema(tool_name)
            schema = json.loads(schema_str)
            if schema is None:
                print(f"\x1b[33mTool '{tool_name}' not found.\x1b[0m")
                return
            print(f"\n\x1b[1m\x1b[36mSchema for: {tool_name}\x1b[0m\n")
            display(JSON(schema))
        except Exception as e:
            self._print_error(f"Error getting schema for '{tool_name}': {e}")

    # ── helpers ───────────────────────────────────────────────────

    def _display_result(self, tool_name: str, result_json: str):
        """Parse and display a tool result."""
        try:
            parsed = json.loads(result_json)
        except json.JSONDecodeError:
            # raw output
            print(result_json)
            return

        if "error" in parsed:
            self._print_error(
                f"Tool '{parsed.get('tool', tool_name)}' error: {parsed['error']}"
            )
            return

        if "ok" in parsed:
            value = parsed["ok"]
            if isinstance(value, str):
                # try to parse as JSON for pretty display
                try:
                    inner = json.loads(value)
                    print(f"\x1b[1m\x1b[36mResult from {tool_name}:\x1b[0m")
                    display(JSON(inner))
                except (json.JSONDecodeError, TypeError):
                    print(f"\x1b[1m\x1b[36mResult from {tool_name}:\x1b[0m")
                    print(value)
            else:
                print(f"\x1b[1m\x1b[36mResult from {tool_name}:\x1b[0m")
                display(JSON(value))
            return

        # fallback: display raw
        print(f"\x1b[1m\x1b[36mResult from {tool_name}:\x1b[0m")
        display(JSON(parsed))

    def _print_error(self, msg: str):
        """Print an error message with red colour."""
        print(f"\x1b[1m\x1b[31mgeo-toolbox error:\x1b[0m {msg}")
