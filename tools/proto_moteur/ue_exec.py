"""Run a Python file inside the open Unreal editor over UE's Remote Execution protocol.

Same channel as the runreal/unreal-mcp server. Usage: python3 ue_exec.py script.py [--timeout S]
"""

import argparse
import sys
import time

sys.path.insert(0, "/Users/Shared/Epic Games/UE_5.7/Engine/Plugins/Experimental/PythonScriptPlugin/Content/Python")
import remote_execution  # noqa: E402


def main() -> int:
    """Discover the editor node, execute the file and print its output."""
    parser = argparse.ArgumentParser()
    parser.add_argument("script")
    parser.add_argument("--timeout", type=float, default=30.0)
    args = parser.parse_args()
    config = remote_execution.RemoteExecutionConfig()
    config.multicast_bind_address = "0.0.0.0"  # macOS drops multicast on sockets bound to 127.0.0.1
    remote = remote_execution.RemoteExecution(config)
    remote.start()
    try:
        deadline = time.time() + args.timeout
        while not remote.remote_nodes:
            if time.time() > deadline:
                print("PROTO no Unreal editor found", file=sys.stderr)
                return 2
            time.sleep(0.5)
        remote.open_command_connection(remote.remote_nodes[0]["node_id"])
        result = remote.run_command(open(args.script).read(), unattended=True, exec_mode=remote_execution.MODE_EXEC_FILE)
        for line in result.get("output", []):
            print(line["output"].rstrip())
        if not result.get("success"):
            print("PROTO failed:", result.get("result"), file=sys.stderr)
            return 1
        return 0
    finally:
        remote.stop()


if __name__ == "__main__":
    sys.exit(main())
