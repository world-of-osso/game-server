# lightyear_messages

- Version: `0.28.0`
- Baseline lockfile checksum: `a183859015aef9270415787c456fcd3d6b1ab7fee1134fde1ddd49befc043e50`
- License: MIT OR Apache-2.0

Based on the installed 0.28.0 source used by the server baseline. This server-local patch adapts transport-free `server-runtime` payload queues to the existing typed native senders and receivers. Message registration, codecs, channel IDs and packet transport remain native. The adapter is opt-in and supports the server's std build. Retire it when upstream exposes a transport-free queue boundary.
